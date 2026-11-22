//! The canonical claim registry + universal fan-out guard (fanout-output
//! US 07). `claim-registry.toml` in the crate maps claim id → owner surface →
//! canonical exact text (or rendered fragments for computed claims); this suite
//! makes the contradiction apparatus structural:
//!
//! - every rendered surface of the built binary (every help topic, every
//!   command's `--help`, `help <command>`, the capability matrix, the skill
//!   output) and every payload doc is scanned for the registry's signature
//!   bytes — an occurrence must be covered by a registered canonical span (the
//!   two allowed relations are byte-identical echo and owner citation, and a
//!   citation carries no signature bytes by definition), or sit in the claim's
//!   owner surface; anything else is a stale echo and fails, naming the claim,
//!   its owner, every location with a line number, and the two allowed fixes
//!   (D07: the message is complete by itself, no instruction doc involved);
//! - the registry is welded to reality: each canonical text is compared against
//!   what the owning surface actually renders or emits (help topics by
//!   rendering them, stdout/stderr contracts by running the commands on
//!   fixtures), and each family's dedicated guard source is parsed and compared
//!   against the registry entry — a stale registry entry or a contradicting
//!   dedicated constant fails here;
//! - planted-echo scenarios run against scratch surfaces (dev-only, never
//!   shipped): a paraphrase, a drifted error quote, a resurrected dead
//!   sentence, and a help-text paraphrase all fail loudly with exact line
//!   numbers; a registry edit that drifts from the rendered canonical fails
//!   the weld; and each of the four contradiction families the blind runs
//!   found stays dead through both its generic scan and its pinned dedicated
//!   guard.

mod common;
#[allow(dead_code)]
mod shared;

use common::{Fixture, stderr, stdout};
use shared::driver::{Driver, Language};
use shared::roles_matrix as matrix;
use shared::scenarios::boundary_spec;

// ---------------------------------------------------------------------------
// registry model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct GuardPin {
    file: String,
    const_name: Option<String>,
    relation: String,
}

#[derive(Debug, Clone)]
struct Claim {
    id: String,
    owner: String,
    owner_surfaces: Vec<String>,
    /// "echo" and/or "cite" — relations a non-owner surface may have.
    relations: Vec<String>,
    canonical: Option<String>,
    canonical_fragments: Vec<String>,
    template: bool,
    signatures: Vec<String>,
    dedicated_guard: Option<String>,
    guard_pins: Vec<GuardPin>,
}

fn manifest() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn load(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", manifest()))
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn registry() -> Vec<Claim> {
    let value: toml::Value = toml::from_str(&load("claim-registry.toml")).expect("registry TOML");
    let entries = value["claim"].as_array().expect("[[claim]] entries");
    entries
        .iter()
        .map(|entry| Claim {
            id: entry["id"].as_str().expect("claim id").to_string(),
            owner: entry["owner"].as_str().expect("claim owner").to_string(),
            owner_surfaces: string_list(&entry["owner_surfaces"]),
            relations: entry
                .get("relations")
                .map_or(vec!["echo".into(), "cite".into()], vec_or_default),
            canonical: entry
                .get("canonical")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            canonical_fragments: string_list_opt(
                &entry
                    .get("canonical_fragments")
                    .cloned()
                    .unwrap_or(toml::Value::Array(vec![])),
            ),
            template: entry
                .get("template")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
            signatures: string_list(&entry["signatures"]),
            dedicated_guard: entry
                .get("dedicated_guard")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            guard_pins: entry
                .get("guard_pin")
                .and_then(|v| v.as_array())
                .map(|pins| {
                    pins.iter()
                        .map(|pin| GuardPin {
                            file: pin["file"].as_str().expect("pin file").to_string(),
                            const_name: pin
                                .get("const_name")
                                .and_then(|v| v.as_str())
                                .map(str::to_string),
                            relation: pin["relation"].as_str().expect("pin relation").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

fn string_list(value: &toml::Value) -> Vec<String> {
    value
        .as_array()
        .expect("string array")
        .iter()
        .map(|v| v.as_str().expect("string entry").to_string())
        .collect()
}

fn string_list_opt(value: &toml::Value) -> Vec<String> {
    value
        .as_array()
        .map(|_| string_list(value))
        .unwrap_or_default()
}

fn vec_or_default(value: &toml::Value) -> Vec<String> {
    string_list_opt(value)
}

// ---------------------------------------------------------------------------
// surfaces and docs
// ---------------------------------------------------------------------------

/// Every text surface an agent can read from the binary: the topic index, each
/// manual topic, each command through both `help <cmd>` and `<cmd> --help`,
/// the capability matrix, and the skill documents (audit and analysis).
fn rendered_surfaces() -> Vec<(String, String)> {
    let fixture = Fixture::new();
    let mut surfaces = vec![("help".to_string(), surface(&fixture, &["help"]))];
    for topic in [
        "commands",
        "glob",
        "spec",
        "constraints",
        "languages",
        "roles",
        "workflow",
        "diagnostics",
    ] {
        surfaces.push((format!("help {topic}"), surface(&fixture, &["help", topic])));
    }
    for command in [
        "init",
        "scan",
        "diagram",
        "verify",
        "explain",
        "update",
        "report",
        "spec",
        "doctor",
        "inspect",
        "depgraph",
        "help",
        "capability",
        "skill",
    ] {
        surfaces.push((
            format!("help {command}"),
            surface(&fixture, &["help", command]),
        ));
        surfaces.push((
            format!("{command} --help"),
            surface(&fixture, &[command, "--help"]),
        ));
    }
    surfaces.push(("skill".to_string(), surface(&fixture, &["skill"])));
    surfaces.push((
        "skill analysis".to_string(),
        surface(&fixture, &["skill", "analysis"]),
    ));
    surfaces.push((
        "capability matrix".to_string(),
        surface(&fixture, &["capability", "matrix"]),
    ));
    surfaces
}

fn surface(fixture: &Fixture, args: &[&str]) -> String {
    let output = fixture.run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`archspec {}` must exit 0: {}",
        args.join(" "),
        stderr(&output)
    );
    stdout(&output)
}

/// The synthetic scan artefact the level accounting welds run against
/// (workplan level_views scenario 01): three raw rows aggregating onto one
/// cross-bucket pair (gateway→store) and one row folding onto the self
/// bucket (gateway→gateway), so the ledger reads `3 drawn + 1 self = 4 raw`.
fn level_weld_artefact(fx: &Fixture) {
    fx.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"alpha","kind":"lib","path":"src/alpha"},{"name":"beta","kind":"lib","path":"src/beta"}],"edges":[],"module_edges":[{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":["a"]},{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":["b"]},{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":[]},{"unit":"alpha","from":"alpha::gateway","to":"beta::gateway","symbols":[]}]}"#,
    );
}

/// Every markdown surface shipped in the payload.
fn payload_docs() -> Vec<(String, String)> {
    let mut files = vec![format!("{}/README.md", manifest())];
    let root = format!("{}/docs", manifest());
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read docs dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path.to_string_lossy().into_owned());
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    files
        .into_iter()
        .map(|path| {
            (
                path.replace(&manifest(), ""),
                std::fs::read_to_string(&path).expect("read payload doc"),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// the generic echo scan: signature occurrences vs canonical spans
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Violation {
    claim_id: String,
    owner: String,
    surface: String,
    line: usize,
    excerpt: String,
    cite_only: bool,
    canonical: String,
    signature: String,
}

/// Whitespace-collapsed per-word record: every word keeps its original line
/// number and its offset in the flattened text, so a signature hit maps back to
/// a line number while checks match across wraps.
struct Flat {
    words: Vec<(String, usize, usize)>, // (lowercased word, line, byte offset)
    text: String,
}

fn flatten_with_lines(text: &str) -> Flat {
    let mut words = Vec::new();
    let mut flat = String::new();
    for (line_no, line) in text.lines().enumerate() {
        for word in line.split_whitespace() {
            let word = word.to_lowercase();
            let start = flat.len();
            if !flat.is_empty() {
                flat.push(' ');
            }
            flat.push_str(&word);
            words.push((word, line_no + 1, start));
        }
    }
    Flat { words, text: flat }
}

fn line_at(flat: &Flat, offset: usize) -> usize {
    match flat
        .words
        .binary_search_by_key(&offset, |(_, _, start)| *start)
    {
        Ok(i) => flat.words[i].1,
        Err(0) => 1,
        Err(i) => flat.words[i - 1].1,
    }
}

fn excerpt_at(text: &str, start: usize) -> String {
    let begin = text[..start]
        .chars()
        .rev()
        .take(70)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    let end = text[start..].chars().take(130).collect::<String>();
    format!("…{begin}{end}…")
}

/// Spans (start, end) in the flattened text where the claim's canonical bytes
/// appear verbatim; templates match their `<path>` placeholder against a
/// wildcard token run.
fn canonical_spans(claim: &Claim, flat: &Flat) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    if let Some(canonical) = &claim.canonical {
        if claim.template {
            let chunks: Vec<String> = canonical
                .split("<path>")
                .map(|chunk| flatten(chunk).to_lowercase())
                .collect();
            let head = chunks.first().expect("template head chunk");
            if !head.is_empty() {
                let mut at = 0;
                while let Some(found) = flat.text[at..].find(head.as_str()) {
                    let start = at + found;
                    let mut cursor = start + head.len();
                    let mut matched = true;
                    for chunk in &chunks[1..] {
                        if chunk.is_empty() {
                            cursor = wildcard_end(&flat.text, cursor);
                            continue;
                        }
                        // Minimal match: the wildcard consumes one token run plus
                        // the single space flatten leaves between tokens; the chunk
                        // must appear at the first position where that holds.
                        let mut next = None;
                        let mut probe = cursor;
                        while let Some(hit) = flat.text[probe..].find(chunk.as_str()) {
                            let candidate = probe + hit;
                            let region = &flat.text[cursor..candidate];
                            let trimmed = region
                                .strip_prefix(' ')
                                .unwrap_or(region)
                                .strip_suffix(' ')
                                .unwrap_or(region);
                            if trimmed.chars().all(token_char) {
                                next = Some(candidate);
                                break;
                            }
                            probe = candidate + 1;
                        }
                        match next {
                            Some(p) => cursor = p + chunk.len(),
                            None => {
                                matched = false;
                                break;
                            }
                        }
                    }
                    if matched {
                        spans.push((start, cursor));
                    }
                    at = start + 1;
                }
            }
        } else {
            let needle = flatten(canonical).to_lowercase();
            let mut at = 0;
            while let Some(found) = flat.text[at..].find(needle.as_str()) {
                spans.push((at + found, at + found + needle.len()));
                at += found + 1;
            }
        }
    }
    for fragment in &claim.canonical_fragments {
        let needle = flatten(fragment).to_lowercase();
        let mut at = 0;
        while let Some(found) = flat.text[at..].find(needle.as_str()) {
            spans.push((at + found, at + found + needle.len()));
            at += found + 1;
        }
    }
    spans
}

/// Characters a `<path>` wildcard may consume: token characters only, so the
/// placeholder stops at whitespace and quotes on its own.
fn token_char(ch: char) -> bool {
    ch.is_alphanumeric()
        || matches!(
            ch,
            '/' | '.' | '-' | '_' | ':' | '<' | '>' | '$' | '@' | '~' | '\\'
        )
}

fn wildcard_end(text: &str, from: usize) -> usize {
    let mut cursor = from;
    for (offset, ch) in text[from..].char_indices() {
        if !token_char(ch) {
            return from + offset;
        }
        cursor = from + offset + ch.len_utf8();
    }
    cursor
}

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Scan one corpus (rendered surfaces and doc surfaces) against the registry.
/// An occurrence of a signature is legal iff it is covered by a canonical span
/// (own claim with an "echo" relation, or a different claim's canonical — a
/// faithful emission of THAT claim), or it sits inside the claim's own owner
/// surface. Anything else is a stale echo.
fn scan(corpus: &[(String, String)], claims: &[Claim]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut spans: Vec<(usize, usize, &str)> = Vec::new();
    for (name, text) in corpus {
        let _ = name;
        let flat = flatten_with_lines(text);
        spans.clear();
        for claim in claims {
            for (start, end) in canonical_spans(claim, &flat) {
                spans.push((start, end, claim.id.as_str()));
            }
        }
        for claim in claims {
            let owner = claim.owner_surfaces.iter().any(|key| key == name);
            for signature in &claim.signatures {
                let needle = flatten(signature).to_lowercase();
                let mut at = 0;
                while let Some(found) = flat.text[at..].find(needle.as_str()) {
                    let start = at + found;
                    let end = start + needle.len();
                    at = start + needle.len().max(1);
                    let boundary_ok = start == 0
                        || !flat.text[..start].ends_with(|c: char| {
                            c.is_alphanumeric() || c == '-' || c == '_' || c == '\''
                        });
                    if !boundary_ok {
                        continue;
                    }
                    let own_span = spans
                        .iter()
                        .any(|(s, e, id)| *s <= start && end <= *e && *id == claim.id);
                    let other_span = spans
                        .iter()
                        .any(|(s, e, id)| *s <= start && end <= *e && *id != claim.id);
                    let echo_ok = own_span && claim.relations.iter().any(|r| r == "echo");
                    if owner || other_span || echo_ok {
                        continue;
                    }
                    violations.push(Violation {
                        claim_id: claim.id.clone(),
                        owner: claim.owner.clone(),
                        surface: name.clone(),
                        line: line_at(&flat, start),
                        excerpt: excerpt_at(&flat.text, start),
                        cite_only: !claim.relations.iter().any(|r| r == "echo"),
                        canonical: claim
                            .canonical
                            .clone()
                            .unwrap_or_else(|| claim.canonical_fragments.join(" / ")),
                        signature: signature.clone(),
                    });
                }
            }
        }
    }
    violations
}

fn render(violations: &[Violation]) -> String {
    let mut out = format!(
        "fan-out guard: {} stale echo(es) of registered claims. A surface restates a \
         registered claim in its own words — the two allowed relations are a byte-identical \
         echo of the canonical text and a citation of the owner surface (a citation carries \
         none of the claim's signature bytes).\n",
        violations.len()
    );
    let mut seen: Vec<&str> = Vec::new();
    for violation in violations {
        if !seen.contains(&violation.claim_id.as_str()) {
            seen.push(&violation.claim_id);
            out.push_str(&format!(
                "\nclaim `{}` — owner: {}.\n",
                violation.claim_id, violation.owner
            ));
        }
        out.push_str(&format!(
            "  {}:{}: signature `{signature}` matched — {excerpt}\n",
            violation.surface,
            violation.line,
            signature = violation.signature,
            excerpt = violation.excerpt,
        ));
        if violation.cite_only {
            out.push_str(&format!(
                "  fix (1): delete the restatement — this claim's canonical bytes belong to the \
                 owner surface alone; fix (2): leave only a citation of the owner: {} — a \
                 citation must contain none of the signature bytes ({signature:?}).\n",
                violation.owner,
                signature = violation.signature,
            ));
        } else {
            out.push_str(&format!(
                "  fix (1): restore the canonical bytes exactly — {} — a byte-identical echo \
                 passes; fix (2): remove the claim here and cite the owner instead: {}.\n",
                violation.canonical, violation.owner,
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// scenario: the registry owns claim identity
// ---------------------------------------------------------------------------

#[test]
fn registry_owns_claim_identity() {
    let claims = registry();
    let mut ids: Vec<&str> = claims.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), claims.len(), "registry claim ids must be unique");
    for claim in &claims {
        assert!(
            !claim.signatures.is_empty(),
            "claim `{}` must carry at least one signature",
            claim.id
        );
        assert!(
            claim.canonical.is_some() || !claim.canonical_fragments.is_empty(),
            "claim `{}` must carry canonical text or canonical fragments",
            claim.id
        );
        assert!(
            !claim.owner.contains(".md") && !claim.owner.contains("docs/"),
            "claim `{}` is owned by a repo-internal doc; owners must be binary surfaces: {}",
            claim.id,
            claim.owner
        );
    }
    // The swept canonical set (blind4 follow-up US 06): every wave's
    // canonical sentence family must hold a registry entry — a sentence with
    // no entry is a plan bug this assert catches, a forked id for one emitted
    // string fails the unique-id assert above.
    let required: [&str; 118] = [
        "audit-recipe",
        "go-module-tier",
        "roles-in-views",
        "facade-inert-note",
        "depgraph-refusal",
        "inspect-refusal",
        "api-usage-role-less-note",
        "depgraph-fold-note",
        "depgraph-async-fold-note",
        "rust-unit-naming",
        "go-naming-legality",
        "report-diagram-granularity",
        "wrote-echo",
        "destination-statement",
        "check-ok-line",
        "exploratory-render",
        "pretty-switch",
        "csharp-duality",
        "fold-pointer",
        "test-gated-edges",
        "mod-label",
        "init-update-summary",
        "check-render-guidance",
        "first-spec-bootstrap",
        "destination-canonical",
        "artefact-dirt-map",
        "files-tier-scan-refusal",
        "files-tier-size-refusal",
        "tsconfig-not-found",
        "tsconfig-load-failed",
        "tsconfig-cycle",
        "package-manifest-invalid",
        "unclassified-resolver-error",
        "external-not-installed",
        "unresolvable-import-misses",
        "scan-census-exit-law",
        "workspace-link-missing",
        "undeclared-member-import",
        "virtual-specifier-not-modelled",
        "framework-conventions-not-configured",
        "workspace-target-not-built",
        "generated-source-not-built",
        "generated-tsconfig-absent",
        "tsconfig-extends-not-installed",
        "ambiguous-root-markers",
        "nested-foreign-marker",
        "ambiguous-markers",
        "dynamic-import-wildcard",
        "ts-vacuity-carriers",
        "kotlin-model-honesty",
        "orphan-file",
        "orphan-file-census",
        // Registered by the npm-channel change (ADR-051): the sentence the
        // agents doc states beside the release-asset guidance.
        "npm-channel-install",
        // Registered by the inspect --clusters overlay (Lane A, 2026-10-23):
        // the four structure verdict labels, the cap-skip law, and the
        // --clusters help-flag row family.
        "clusters-structure-verdicts",
        "clusters-cap-skip",
        "clusters-help-flags",
        // Registered by the issue-9 violations pointer (2026-11-05): the
        // sentence routing wrong-edge questions from the depgraph help
        // surfaces to the diagram scan-mode marked view.
        "depgraph-violations-pointer",
        // Registered by the issue-19 evidence-honesty law (2026-11-08): the
        // finding class naming a pair with code import evidence and no
        // declared edge, and the suppression of the zero-evidence reason.
        "used-but-undeclared-import",
        "async-event-unresolved",
        "async-publish-not-derived",
        "async-subscribe-not-derived",
        "async-endpoint-unclaimed",
        "async-cycle-advisory",
        // Registered by GitHub issue #48 rung P2 (2026-11-18): the
        // shared-simple-name collision census row of the identity-keyed
        // async render set.
        "async-event-collision-advisory",
        "async-depends-inert-notes",
        "async-drift-field-absent",
        "async-drift-enum-projection",
        // Registered by the archspec_async_depends scenario 05 (2026-11-17):
        // the ONE async census section of the report renderers and the
        // verify --json machine body, rendered iff the declared table
        // resolves a pair.
        "async-census-section",
        // Registered by the events_value_join scenario 02 (2026-11-19): the
        // declaration-face refusal classes, its census section, and the
        // shared-constant pattern-pair load refusal (ADR-069 4.4a).
        "declaration-producer-unresolved",
        "declaration-producer-not-canonical",
        "declaration-producer-conflict",
        "declaration-census-section",
        "shared-constant-pattern-pair-refusal",
        // Registered by the literal-site pattern primitives (workplan
        // events_value_join scenario 03): the three generic matcher verbs, the
        // scan-report census lines and the named driver-gap absence row.
        "literal-site-pattern-primitives",
        // Registered by the value join (workplan events_value_join scenario 04):
        // the derived markers, the PLUGIN-OVERLAP merge row and the named
        // zero-silence row set of the FIRST async_edges construction site.
        "value-join-rows",
        "attribution-census-rows",
        // Registered by the dispatch-blinded projection (workplan
        // events_value_join scenario 06, F-W4): the ONE report-only census
        // row per zone whose 4.8 dispatch row dispatches a declared
        // pattern language out — a projection over the shipped rows.
        "dispatch-blinded-literals",
        // Registered by the issue-8 fail-closed law (2026-11-05): the one
        // shared key-shape table behind the emitted schema and the runtime
        // unknown-key refusal.
        "spec-schema-fail-closed-shape",
        // Registered by GitHub issue #25 (2026-11-08): the `--json` flag line
        // of the verify help — the machine form of the verdict.
        "verify-help-json-flag",
        "verdict-environment-declared",
        "verify-only-class-filter",
        // Registered by GitHub issue #26 (2026-11-08): the renderer claim
        // behind the explain verb — the trace prints what the verify path
        // computed, never a re-derivation.
        "explain-renderer",
        // Registered by GitHub issue #27 (2026-11-09): the [[exempt]] ratchet
        // — the seed-only ledger flag line of the verify help and the
        // stale/expired warning pair of the diagnostics catalog.
        "verify-help-seed-flag",
        "exempt-ratchet-diagnostics",
        // Registered by GitHub issue #39 (2026-11-16): the csharp test-gated
        // census — the per-project line and the verdict note for dropped
        // test projects.
        "csharp-test-gated-units",
        // Registered by GitHub issue #45 rung 1 (2026-11-18): the csharp
        // conditioned test-marker suppression line — a test marker under an
        // unevaluated MSBuild Condition gates nothing and says so.
        "csharp-conditioned-test-markers",
        // Registered by workplan csharp_condition_eval scenario 04
        // (2026-11-21, the F2 rider): the condition-face gap rows — the
        // `Directory.Build.targets` files and `Import` targets the csharp
        // driver does not walk name themselves, presence only.
        "csharp-condition-face-unwalked",
        // Registered by GitHub issue #49 (2026-11-19): the csharp declared
        // exclusion — the per-project census line of `excluded_sources` at
        // project discovery.
        "csharp-excluded-units",
        // Registered by GitHub issue #60 (2026-11-19): the csharp declared
        // exclusion at the FILE walk — the per-source-file census line of
        // `excluded_sources` inside a kept project.
        "csharp-excluded-sources",
        // Registered by workplan visibility_phase2 scenario 03 (2026-11-20):
        // the manifest path-item rows (the five named skip classes of the
        // name-free item walk) and the shared-source census row.
        "csharp-manifest-path-items",
        "csharp-shared-source-census",
        // Registered by GitHub issue #28 (2026-11-17): the import-order repro
        // emitter — the verify help flag line, the repro block per runtime
        // file ring, and the named skip row of a language without the
        // one-liner form.
        "verify-help-repro-flag",
        "cycle-repro-lines",
        "cycle-repro-language-skip",
        // Registered by GitHub issue #54 (2026-11-18): the verdict-aware
        // verify headline — a passing run never prints the mismatch headline —
        // and the --strict help sentence naming the async census-row exception.
        "verify-verdict-headline",
        "verify-help-strict-flag",
        // Registered by workplan visibility_phase2 scenario 05 (2026-11-20):
        // the `--layer` refusal bytes and the ceiling rationale the diagram
        // help owns.
        "layer-selector-refusal",
        "layer-ceiling-rationale",
        // Registered by the improvement-sweep HIGH-1 cure (2026-11-20): the
        // ceiling rationale armed at the mermaid emit seam — the refusal row
        // every over-ceiling `--format mermaid` render prints at exit 64.
        "mermaid-ceiling-refusal",
        // Registered by workplan visibility_phase2 scenario 06 (2026-11-20):
        // the single-file viewer law and the viewer channels/limitation the
        // diagram help owns.
        "viewer-single-file-law",
        "viewer-channels-limitation",
        // Registered by workplan viewer_usability scenario 01 (2026-11-22):
        // the emitted document's interaction surface — declared-deps class,
        // selection clear, viewBox-only pan/zoom, keyboard/ARIA affordances.
        "viewer-interaction-controls",
        // Registered by workplan level_views scenario 01 (2026-11-20): the
        // `--level` refusal bytes and the accounting block its renders print.
        "level-selector-refusal",
        "level-dir-floor-refusal",
        "level-source-refusal",
        "level-tiers-refusal",
        "level-descriptive-note",
        "level-accounting-exact",
        "level-empty-model",
        "level-completeness-restatement",
        // Registered by workplan level_views scenario 03 (2026-11-21): the
        // declared-vs-projected unit delta census `--level unit` prints
        // advisory-LOUD on stderr (rc unchanged).
        "level-unit-delta-model-only",
        "level-unit-delta-projection-only",
        // Registered by workplan level_views scenario 02 (2026-11-21): the
        // NAME-THE-FITS refusal rows on the over-ceiling level render and
        // the two-step narrowing grammar.
        "level-fits-listing",
        "level-empty-fits-row",
        "level-narrowing-grammar",
        // Registered by workplan level_views scenario 04 (2026-11-21): the
        // recursive panes — the in-pane TERMINATION refusal and the
        // whole-document total row of the level-viewer document.
        "level-pane-refusal",
        "level-document-total",
        "viewer-byte-ceiling-refusal",
    ];
    for required in required {
        assert!(
            claims.iter().any(|c| c.id == required),
            "the registry must hold the `{required}` family",
        );
    }
    assert_eq!(
        claims.len(),
        required.len(),
        "the registry entry count must equal the swept canonical set: {} entries against {} required families — a forked or unregistered sentence is a plan bug",
        claims.len(),
        required.len()
    );
    let keys: Vec<String> = rendered_surfaces()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    for claim in &claims {
        for key in &claim.owner_surfaces {
            assert!(
                keys.contains(key),
                "claim `{}` names owner surface {key:?} that the binary does not render",
                claim.id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// scenario: the registry is welded to what the surfaces render and emit,
// and to what the dedicated guards pin
// ---------------------------------------------------------------------------

fn check_weld(claims: &[Claim]) -> Vec<String> {
    let mut errors = Vec::new();
    let surfaces = rendered_surfaces();
    // The pane-explosion mass: 120 directories, each a 24-row internal
    // grid. The UNIT fold aggregates the rows into cross-rows (a tiny
    // document); the finer folds draw a pane per directory — over the
    // viewer emitted-byte ceiling (workplan level_views scenario 05). The
    // largest-fitting-scope weld runs against it, as do the cells of
    // `tests/level_viewer_bytes.rs`.
    let ceiling_fixture = {
        let fx = Fixture::new();
        let mut edges: Vec<String> = Vec::new();
        for dir in 0..120 {
            for row in 0..24 {
                edges.push(format!(
                    "{{\"unit\":\"model\",\"from\":\"w/d{dir}/g{row}/f.rs\",\"to\":\"w/d{dir}/h{row}/f.rs\",\"symbols\":[]}}"
                ));
            }
        }
        fx.write(
            "model.json",
            &format!(
                "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"w\",\"kind\":\"lib\",\"path\":\"w\"}}],\"edges\":[],\"module_edges\":[{}]}}",
                edges.join(",")
            ),
        );
        fx
    };
    let rendered = |key: &str| -> String {
        surfaces
            .iter()
            .find(|(name, _)| name == key)
            .unwrap_or_else(|| panic!("surface {key:?} renders"))
            .1
            .clone()
    };
    let contains_canonical = |errors: &mut Vec<String>, claim: &Claim, key: &str| {
        let text = flatten(&rendered(key)).to_lowercase();
        for fragment in &claim.canonical_fragments {
            if !text.contains(&flatten(fragment).to_lowercase()) {
                errors.push(format!(
                    "claim `{}` drifted: `archspec {key}` does not render the registered line {fragment:?}",
                    claim.id
                ));
            }
        }
        if let Some(canonical) = &claim.canonical
            && !claim.template
            && !text.contains(&flatten(canonical).to_lowercase())
        {
            errors.push(format!(
                    "claim `{}` drifted: `archspec {key}` does not render the registered canonical text",
                    claim.id
                ));
        }
    };
    for claim in claims {
        match claim.id.as_str() {
            "audit-recipe" => contains_canonical(&mut errors, claim, "help workflow"),
            "go-module-tier" | "rust-unit-naming" | "go-naming-legality" => {
                contains_canonical(&mut errors, claim, "help languages")
            }
            "report-diagram-granularity" => contains_canonical(&mut errors, claim, "report --help"),
            "roles-in-views" => contains_canonical(&mut errors, claim, "help roles"),
            "depgraph-refusal" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let fx = Fixture::new();
                fx.write("go.mod", "module example.com/solo\n\ngo 1.21\n");
                fx.write("pkg/solo.go", "package pkg\n\nconst X = 1\n");
                let refused = fx.run(&["depgraph", "modules"]);
                if refused.status.code() != Some(1)
                    || !stderr(&refused).contains(canonical.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: a tier-less tree's live refusal bytes differ from the registered canonical: {}",
                        claim.id,
                        stderr(&refused)
                    ));
                }
            }
            "inspect-refusal" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let fx = Fixture::new();
                fx.write("go.mod", "module example.com/solo\n\ngo 1.21\n");
                fx.write("pkg/solo.go", "package pkg\n\nconst X = 1\n");
                let refused = fx.run(&["inspect", "tree"]);
                if refused.status.code() != Some(1)
                    || !stderr(&refused).contains(canonical.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: a tier-less tree's live inspect refusal bytes differ from the registered canonical: {}",
                        claim.id,
                        stderr(&refused)
                    ));
                }
            }
            "files-tier-scan-refusal" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let guard = std::fs::read_to_string(format!(
                    "{}/tests/diagram_files_tier.rs",
                    manifest()
                ))
                .expect("dedicated guard exists");
                if !flatten(&guard).contains(&flatten(&canonical)) {
                    errors.push(format!(
                        "claim `{}` drifted: the dedicated guard no longer states the registered refusal bytes",
                        claim.id
                    ));
                }
                let fx = Fixture::new();
                fx.write("package.json", "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n");
                fx.write("src/a.ts", "export const a = 1;\n");
                let scanned = fx.run(&["scan", ".", "--output", "scan.json"]);
                assert!(scanned.status.code() == Some(0), "probe scan artefact: {}", stderr(&scanned));
                let refused = fx.run(&["diagram", "--source", "scan", "scan.json", "--tiers", "files"]);
                if refused.status.code() != Some(64)
                    || !stderr(&refused).contains(canonical.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: the scan-source files-tier refusal bytes differ from the registered canonical — exit {:?}, stderr {:?}",
                        claim.id,
                        refused.status.code(),
                        stderr(&refused)
                    ));
                }
            }
            "files-tier-size-refusal" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let fx = Fixture::new();
                fx.write("package.json", "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n");
                fx.write("src/a.ts", "export const a = 1;\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n",
                );
                let refused = fx.run(&["diagram", ".", "--tiers", "files"]);
                if refused.status.code() != Some(64)
                    || !stderr(&refused).contains(canonical.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: the unscoped files-tier refusal bytes differ from the registered canonical — exit {:?}, stderr {:?}",
                        claim.id,
                        refused.status.code(),
                        stderr(&refused)
                    ));
                }
            }
            "facade-inert-note" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let fx = Fixture::new();
                fx.write("go.mod", "module example.com/probe/go\n\ngo 1.21\n");
                fx.write(
                    "app/app.go",
                    "package app\n\nimport \"example.com/probe/go/shared\"\n\nfunc Run() { shared.X() }\n",
                );
                fx.write("shared/shared.go", "package shared\n\nconst X = 1\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"example.com/probe/go/app\"] }\n[module.allowed]\ndepend_on = [\"shared\"]\n\n[[module]]\nname = \"shared\"\nmatches = { units = [\"example.com/probe/go/shared\"] }\n",
                );
                let run = fx.run(&["verify"]);
                if run.status.code() != Some(0) || !stdout(&run).contains(canonical.as_str()) {
                    errors.push(format!(
                        "claim `{}` drifted: a go verify run does not emit the registered inert note verbatim: {}",
                        claim.id,
                        stdout(&run)
                    ));
                }
            }
            "api-usage-role-less-note" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let view = matrix::VIEWS
                    .iter()
                    .find(|v| v.name == "depgraph api-usage")
                    .expect("api-usage view is registered in the matrix");
                if view.self_statement != Some(canonical.as_str()) {
                    errors.push(format!(
                        "claim `{}` contradicts the matrix registry: the view's self_statement differs from the canonical note",
                        claim.id
                    ));
                }
                let fx = Fixture::new();
                matrix::materialize("rust-libbin", &fx);
                let out = fx.run(&["depgraph", "api-usage"]);
                if out.status.code() != Some(0) || !stdout(&out).contains(canonical.as_str()) {
                    errors.push(format!(
                        "claim `{}` drifted: `depgraph api-usage` does not emit the registered note line verbatim: {}",
                        claim.id,
                        stdout(&out)
                    ));
                }
            }
            "depgraph-fold-note" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let (head, tail) = canonical
                    .split_once("<path>")
                    .expect("the fold note is a templated canonical");
                let fx = Fixture::new();
                // Single-word project names with corporate `RootNamespace`
                // spellings (GitHub issue #61 made dotted project names
                // render verbatim, so they can no longer weld a truncating
                // fold): the two root keys still fold onto one `Api` node.
                let csproj = |root_ns: &str, reference: &str| {
                    format!(
                        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n    <RootNamespace>{root_ns}</RootNamespace>\n  </PropertyGroup>\n{}{}</Project>\n",
                        if reference.is_empty() { "" } else { "  <ItemGroup>\n" },
                        if reference.is_empty() {
                            String::new()
                        } else {
                            format!("    <ProjectReference Include=\"{reference}\" />\n  </ItemGroup>\n")
                        }
                    )
                };
                fx.write(
                    "Alpha/Alpha.csproj",
                    &csproj("Alpha.Api", "../Shared/Shared.csproj"),
                );
                fx.write("Alpha/Wiring.cs", "using Shared.Lib;\nnamespace Alpha.Api;\n");
                fx.write(
                    "Beta/Beta.csproj",
                    &csproj("Beta.Api", "../Shared/Shared.csproj"),
                );
                fx.write(
                    "Beta/Program.cs",
                    "using Microsoft.Extensions.DependencyInjection;\nusing Shared.Lib;\nnamespace Beta.Api;\npublic static class Program\n{\n    public static void Main()\n    {\n        var services = new ServiceCollection();\n        services.AddScoped<ICache, Cache>();\n    }\n}\n",
                );
                fx.write("Shared/Shared.csproj", &csproj("Shared.Lib", ""));
                fx.write(
                    "Shared/Shared.cs",
                    "namespace Shared.Lib;\npublic interface ICache { }\npublic class Cache : ICache { }\n",
                );
                let out = fx.run(&["depgraph", "modules"]);
                // The conflicting fold (facade + composition onto one `Api`
                // node) must emit the note with its claim count — two.
                let expected = format!("{head}2{tail}");
                if out.status.code() != Some(0) || !stdout(&out).contains(expected.as_str()) {
                    errors.push(format!(
                        "claim `{}` drifted: a conflicting fold does not append the registered note with its claim count verbatim: {}",
                        claim.id,
                        stdout(&out)
                    ));
                }
            }
            "depgraph-async-fold-note" => {
                let canonical = claim.canonical.clone().expect("canonical");
                let (head, rest) = canonical
                    .split_once("<path>")
                    .expect("the async fold note is a templated canonical");
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[workspace]\nmembers = [\"alpha\", \"beta\", \"gamma\"]\nresolver = \"2\"\n",
                );
                for unit in ["alpha", "beta", "gamma"] {
                    fx.write(
                        &format!("{unit}/Cargo.toml"),
                        &format!("[package]\nname = \"{unit}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
                    );
                }
                fx.write("alpha/src/lib.rs", "pub mod gateway;\n");
                fx.write("alpha/src/gateway.rs", "pub fn publish() {}\n");
                fx.write("beta/src/lib.rs", "pub mod gateway;\n");
                fx.write("beta/src/gateway.rs", "pub fn consume() {}\n");
                fx.write("gamma/src/lib.rs", "pub mod admin;\n");
                fx.write("gamma/src/admin.rs", "pub fn panel() {}\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"alpha\"\nmatches = { units = [\"alpha\"] }\n\n[[module]]\nname = \"beta\"\nmatches = { units = [\"beta\"] }\n\n[[module]]\nname = \"gamma\"\nmatches = { units = [\"gamma\"] }\n\n[[async_depends]]\nfrom = \"alpha\"\nto = \"beta\"\nevent = \"ping::PingEvent\"\n",
                );
                let out = fx.run(&["depgraph", "modules"]);
                // The fold-to-self pair places one node per endpoint and
                // folds the row onto a self-edge: the note rides stderr with
                // its count and NO endpoint tail, never silence (issue #51).
                let head_note = format!(
                    "{head}1{}",
                    rest.split(" — endpoints").next().expect("the note head")
                );
                let err = stderr(&out);
                if out.status.code() != Some(0) || !err.contains(head_note.as_str()) {
                    errors.push(format!(
                        "claim `{}` drifted: a folded-away async row does not emit the registered note with its count verbatim: {}",
                        claim.id, err
                    ));
                }
                if err.contains("endpoints outside") {
                    errors.push(format!(
                        "claim `{}` drifted: a fold-to-self run printed the endpoint tail although every endpoint placed a node: {}",
                        claim.id, err
                    ));
                }
            }
            "wrote-echo" => {
                let fx = Fixture::new();
                let driver = Driver {
                    language: Language::Rust,
                };
                driver.materialize(&fx, &driver.probe_tree());
                fx.write("architecture.spec.toml", &boundary_spec(&driver));
                fx.write(
                    "archspec.toml",
                    "[output]\nreport = \"out.md\"\nscan = \"model.json\"\n",
                );
                let out = fx.run(&["report"]);
                if out.status.code() != Some(0) || stdout(&out) != "wrote out.md\n" {
                    errors.push(format!(
                        "claim `{}` drifted: a destination write must echo exactly `wrote <path>`, stdout was {:?}",
                        claim.id,
                        stdout(&out)
                    ));
                }
                // The echo is universal on the write axis (blind4 US 01): the
                // machine destination and an explicit `--output` echo alike.
                let machine = fx.run(&["scan"]);
                if machine.status.code() != Some(0) || stdout(&machine) != "wrote model.json\n" {
                    errors.push(format!(
                        "claim `{}` drifted: a machine destination write must echo exactly `wrote <path>`, stdout was {:?}",
                        claim.id,
                        stdout(&machine)
                    ));
                }
                let flagged = fx.run(&["depgraph", "modules", "--output", "dg.mmd"]);
                if flagged.status.code() != Some(0) || stdout(&flagged) != "wrote dg.mmd\n" {
                    errors.push(format!(
                        "claim `{}` drifted: an explicit `--output` write must echo exactly `wrote <path>`, stdout was {:?}",
                        claim.id,
                        stdout(&flagged)
                    ));
                }
            }
            "destination-statement" => {
                let fx = Fixture::new();
                let driver = Driver {
                    language: Language::Rust,
                };
                driver.materialize(&fx, &driver.probe_tree());
                fx.write("architecture.spec.toml", &boundary_spec(&driver));
                fx.write("archspec.toml", "[output]\nreport = \"out.md\"\n");
                fx.write("out.md", "provisioned\n");
                let out = fx.run(&["report", "--format", "markdown"]);
                let first = stdout(&out).lines().next().unwrap_or_default().to_string();
                if out.status.code() != Some(0) || first != "note: destination out.md not written" {
                    errors.push(format!(
                        "claim `{}` drifted: a `--format` bypass must open stdout with `note: destination <path> not written`, first line was {first:?}",
                        claim.id,
                    ));
                }
            }
            "check-ok-line" => {
                contains_canonical(&mut errors, claim, "scan --help");
                let fx = Fixture::new();
                let driver = Driver {
                    language: Language::Rust,
                };
                driver.materialize(&fx, &driver.probe_tree());
                fx.write("architecture.spec.toml", &boundary_spec(&driver));
                fx.write("archspec.toml", "[output]\nreport = \"out.md\"\n");
                let written = fx.run(&["report"]);
                if written.status.code() != Some(0) {
                    errors.push(format!(
                        "claim `{}` weld: the fresh check needs a provisioned destination first: {}",
                        claim.id,
                        stderr(&written)
                    ));
                }
                let checked = fx.run(&["report", "--check"]);
                if checked.status.code() != Some(0) || stdout(&checked) != "ok: out.md up to date\n" {
                    errors.push(format!(
                        "claim `{}` drifted: a fresh `--check` must print exactly `ok: <path> up to date` on stdout and exit 0, stdout was {:?} exit {:?}",
                        claim.id,
                        stdout(&checked),
                        checked.status.code()
                    ));
                }
            }
            "exploratory-render" => {
                contains_canonical(&mut errors, claim, "diagram --help");
                let fx = Fixture::new();
                let driver = Driver {
                    language: Language::Rust,
                };
                driver.materialize(&fx, &driver.probe_tree());
                fx.write("architecture.spec.toml", &boundary_spec(&driver));
                fx.write("archspec.toml", "[output]\ndiagram = \"diagram.mmd\"\n");
                fx.write("diagram.mmd", "COMMITTED SPEC-MODE DIAGRAM\n");
                let scanned = fx.run(&["scan", "--output", "model.json"]);
                if scanned.status.code() != Some(0) {
                    errors.push(format!(
                        "claim `{}` weld: the scan artefact of the exploratory probe must be written: {}",
                        claim.id,
                        stderr(&scanned)
                    ));
                }
                let exploratory = fx.run(&["diagram", "--source", "scan", "model.json"]);
                let first = stdout(&exploratory)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                if exploratory.status.code() != Some(0)
                    || first != "note: destination diagram.mmd not written"
                    || !stdout(&exploratory).contains("graph")
                    || fx.read("diagram.mmd") != "COMMITTED SPEC-MODE DIAGRAM\n"
                {
                    errors.push(format!(
                        "claim `{}` drifted: an exploratory `--source scan` render without `--output` must open stdout with the destination-not-written note, carry the body, and leave the destination byte-identical — first line {first:?}, exit {:?}",
                        claim.id,
                        exploratory.status.code()
                    ));
                }
                // the exploratory rule holds under the tier flag too (workplan
                // workplan_archspec_scan_diagram_module_tier, D8: destination and
                // check semantics are unchanged in either flag state).
                let flagged = fx.run(&[
                    "diagram",
                    "--source",
                    "scan",
                    "model.json",
                    "--tiers",
                    "units,modules",
                ]);
                let flagged_first = stdout(&flagged)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                if flagged.status.code() != Some(0)
                    || flagged_first != "note: destination diagram.mmd not written"
                    || !stdout(&flagged).contains("graph")
                    || fx.read("diagram.mmd") != "COMMITTED SPEC-MODE DIAGRAM\n"
                {
                    errors.push(format!(
                        "claim `{}` drifted: an exploratory render with `--tiers` must keep the same note, body and untouched destination — first line {flagged_first:?}, exit {:?}",
                        claim.id,
                        flagged.status.code()
                    ));
                }
            }
            "pretty-switch" => {
                for (fragment, key) in claim.canonical_fragments.iter().zip(["scan --help", "report --help"]) {
                    let text = flatten(&rendered(key)).to_lowercase();
                    if !text.contains(&flatten(fragment).to_lowercase()) {
                        errors.push(format!(
                            "claim `{}` drifted: `archspec {key}` does not render the registered line {fragment:?}",
                            claim.id
                        ));
                    }
                }
                let fx = Fixture::new();
                let driver = Driver {
                    language: Language::Rust,
                };
                driver.materialize(&fx, &driver.probe_tree());
                let compact = fx.run(&["scan"]);
                let pretty = fx.run(&["scan", "--pretty"]);
                if pretty.status.code() != Some(0)
                    || stdout(&compact).trim().lines().count() != 1
                    || stdout(&pretty).lines().count() < 2
                {
                    errors.push(format!(
                        "claim `{}` drifted: `scan` must default to one compact line and `scan --pretty` must emit the same model JSON indented",
                        claim.id
                    ));
                }
            }
            "csharp-duality" => contains_canonical(&mut errors, claim, "help languages"),
            "kotlin-model-honesty" => contains_canonical(&mut errors, claim, "help languages"),
            "fold-pointer" => {
                contains_canonical(&mut errors, claim, "depgraph --help");
                contains_canonical(&mut errors, claim, "help depgraph");
                // The position pin (blind5 follow-up US 05): US 02 moved the
                // relocated sentence ahead of the `mod`-label paragraph and
                // welded its bytes; this leg owns the order fact — a stale
                // surface that parks it back in the footnotes fails here.
                for key in ["depgraph --help", "help depgraph"] {
                    let text = flatten(&rendered(key)).to_lowercase();
                    let order = text
                        .find("node labels fold units")
                        .zip(text.find("the `mod` label is a unit's root-module bucket"));
                    match order {
                        Some((fold_at, mod_at)) if fold_at < mod_at => {}
                        _ => errors.push(format!(
                            "claim `{}` drifted: `archspec {key}` no longer states the fold sentence before the `mod`-label paragraph",
                            claim.id
                        )),
                    }
                }
            }
            "test-gated-edges" | "mod-label" => {
                contains_canonical(&mut errors, claim, "depgraph --help")
            }
            "depgraph-violations-pointer" => {
                contains_canonical(&mut errors, claim, "depgraph --help");
                contains_canonical(&mut errors, claim, "help depgraph");
            }
            "init-update-summary" => {
                contains_canonical(&mut errors, claim, "help");
                contains_canonical(&mut errors, claim, "help commands");
            }
            "check-render-guidance" => {
                let canonical = claim.canonical.clone().expect("guidance canonical");
                let needle = flatten(&canonical).to_lowercase();
                for key in ["report --help", "help report", "help commands"] {
                    if !flatten(&rendered(key)).to_lowercase().contains(&needle) {
                        errors.push(format!(
                            "claim `{}` drifted: `archspec {key}` does not render the registered guidance sentence",
                            claim.id
                        ));
                    }
                }
                // The welded statement surface (D02): the config doc states
                // the same sentence byte-identically beside the [output] rules.
                if !flatten(&load("docs/archspec/config.md"))
                    .to_lowercase()
                    .contains(&needle)
                {
                    errors.push(format!(
                        "claim `{}` drifted: the config doc's `[output]` section no longer states the registered guidance sentence byte-identically",
                        claim.id
                    ));
                }
                // The emitted error bytes: a live stale check of each of the
                // three report renderings must carry the registered branch
                // fragments (fragments: advice clause, text, markdown, json).
                let branches = <[String; 4]>::try_from(claim.canonical_fragments.clone())
                    .unwrap_or_else(|fragments| {
                        panic!(
                            "claim `{}` must register exactly the advice clause plus the three check-error branches, saw {fragments:?}",
                            claim.id
                        )
                    });
                let [advice, text_branch, markdown_branch, json_branch] = branches;
                let fx = Fixture::new();
                let driver = Driver {
                    language: Language::Rust,
                };
                driver.materialize(&fx, &driver.probe_tree());
                fx.write("architecture.spec.toml", &boundary_spec(&driver));
                fx.write("archspec.toml", "[output]\nreport = \"out.md\"\n");
                let written = fx.run(&["report"]);
                if written.status.code() != Some(0) {
                    errors.push(format!(
                        "claim `{}` weld: the check-error probe needs a provisioned text destination first: {}",
                        claim.id,
                        stderr(&written)
                    ));
                } else {
                    fx.write("out.md", &format!("{}\nrestaled\n", fx.read("out.md")));
                    let stale = fx.run(&["report", "--check"]);
                    if stale.status.code() != Some(1)
                        || !stderr(&stale).contains(text_branch.as_str())
                        || !stderr(&stale).contains(advice.as_str())
                    {
                        errors.push(format!(
                            "claim `{}` drifted: the stale canonical-rendering check must exit 1 and emit the registered branch bytes {text_branch:?} (advice {advice:?}) — exit {:?}, stderr {:?}",
                            claim.id,
                            stale.status.code(),
                            stderr(&stale)
                        ));
                    }
                    for (format, branch) in
                        [("markdown", &markdown_branch), ("json", &json_branch)]
                    {
                        let stale = fx.run(&["report", "--format", format, "--check"]);
                        if stale.status.code() != Some(1)
                            || !stderr(&stale).contains(branch.as_str())
                            || stderr(&stale).contains(advice.as_str())
                        {
                            errors.push(format!(
                                "claim `{}` drifted: the stale `--format {format}` check must exit 1 and emit the registered branch bytes {branch:?} without the default-rendering advice — exit {:?}, stderr {:?}",
                                claim.id,
                                stale.status.code(),
                                stderr(&stale)
                            ));
                        }
                    }
                }
            }
            "first-spec-bootstrap" => contains_canonical(&mut errors, claim, "help workflow"),
            "destination-canonical" => {
                for key in ["report --help", "help report", "help commands"] {
                    contains_canonical(&mut errors, claim, key);
                }
            }
            "artefact-dirt-map" => contains_canonical(&mut errors, claim, "help diagnostics"),
            // The csharp manifest walk (workplan visibility_phase2
            // scenario 03): the skip-class row grammars and the
            // shared-source census row are the diagnostics catalog rows;
            // the dedicated guard welds them to the live scanner output.
            "csharp-manifest-path-items" | "csharp-shared-source-census" => {
                contains_canonical(&mut errors, claim, "help diagnostics")
            }
            "tsconfig-not-found"
            | "tsconfig-load-failed"
            | "tsconfig-cycle"
            | "package-manifest-invalid"
            | "unclassified-resolver-error"
            | "external-not-installed"
            | "unresolvable-import-misses"
            | "workspace-link-missing"
            | "undeclared-member-import"
            | "used-but-undeclared-import"
            | "virtual-specifier-not-modelled"
            | "framework-conventions-not-configured"
            | "workspace-target-not-built"
            | "generated-source-not-built"
            | "generated-tsconfig-absent"
            | "tsconfig-extends-not-installed"
            | "ambiguous-root-markers"
            | "nested-foreign-marker"
            | "ambiguous-markers"
            | "dynamic-import-wildcard"
            | "orphan-file"
            | "orphan-file-census"
            | "csharp-test-gated-units"
            | "csharp-conditioned-test-markers"
            | "csharp-condition-face-unwalked"
            | "csharp-excluded-units"
            | "csharp-excluded-sources"
            | "async-event-unresolved"
            | "async-publish-not-derived"
            | "async-subscribe-not-derived"
            | "async-drift-field-absent"
            | "async-drift-enum-projection"
            | "async-endpoint-unclaimed"
            | "async-cycle-advisory"
            | "async-event-collision-advisory"
            | "declaration-producer-unresolved"
            | "declaration-producer-not-canonical"
            | "declaration-producer-conflict" => {
                contains_canonical(&mut errors, claim, "help diagnostics")
            }
            // HIGH-2 src part (2026-11-20 sweep): the scan census exit
            // class on its surfaces — fragment[0] is the `scan --help`
            // trailer (echoed by `help scan`), fragment[1] the Exit law
            // paragraph of the `unresolvable import` catalog entry.
            "scan-census-exit-law" => {
                for (fragment, keys) in claim.canonical_fragments.iter().zip([
                    &["scan --help", "help scan"][..],
                    &["help diagnostics"][..],
                ]) {
                    for key in keys {
                        let text = flatten(&rendered(key)).to_lowercase();
                        if !text.contains(&flatten(fragment).to_lowercase()) {
                            errors.push(format!(
                                "claim `{}` drifted: `archspec {key}` does not render the registered line {fragment:?}",
                                claim.id
                            ));
                        }
                    }
                }
            }
            // The declaration-face census (workplan events_value_join
            // scenario 02): live probe, both sides — a rust run whose
            // `[[shared_constant]]` table resolves one entry renders the
            // section header on the report surface; a run with neither
            // declaration table renders no census bytes at all (D8).
            "declaration-census-section" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"declweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub const PROTOCOL_VERSION: u32 = 4;\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"declweld\"\nmatches = { units = [\"declweld\"] }\n\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \".\"\n",
                );
                let declaring = fx.run(&["report"]);
                if !stdout(&declaring).contains("Declaration census") {
                    errors.push(format!(
                        "claim `{}` drifted: a declaring rust report does not render the section header: {}",
                        claim.id,
                        stdout(&declaring)
                    ));
                }
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"declweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub fn api() {}\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"declweld\"\nmatches = { units = [\"declweld\"] }\n",
                );
                let non_declaring = fx.run(&["report"]);
                if stdout(&non_declaring).contains("Declaration census") {
                    errors.push(format!(
                        "claim `{}` drifted: a non-declaring run renders census bytes (D8): {}",
                        claim.id,
                        stdout(&non_declaring)
                    ));
                }
            }
            // The F-W3 load-time refusal (workplan events_value_join
            // scenario 02): a live run whose `[[shared_constant]]` entry
            // names the `emit` key exits 1 with the registered message
            // bytes, its own class, never the unknown-key echo.
            "shared-constant-pattern-pair-refusal" => {
                // Live bytes from the registered template: the weld runs
                // the `emit` key at entry #1, so the placeholders resolve
                // to those concrete tokens here.
                let canonical = claim
                    .canonical
                    .clone()
                    .expect("canonical")
                    .replace("<key>", "emit")
                    .replace("entry #N", "entry #1");
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"pairweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub const PROTOCOL_VERSION: u32 = 4;\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"pairweld\"\nmatches = { units = [\"pairweld\"] }\n\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \".\"\nemit = \"publish\"\n",
                );
                let refused = fx.run(&["verify"]);
                if refused.status.code() != Some(1)
                    || !stderr(&refused).contains(canonical.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: the pattern-pair load refusal bytes differ from the registered canonical — exit {:?}, stderr {:?}",
                        claim.id,
                        refused.status.code(),
                        stderr(&refused)
                    ));
                }
            }
            "literal-site-pattern-primitives" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
                // Live probe (workplan events_value_join scenario 03): a ts
                // fixture whose declared emit side matches one call site
                // prints the registered census line with the placeholders
                // resolved — the never-silent funded arm — and a run without
                // the table prints no census bytes at all (4.2 arm 1).
                let line = claim.canonical_fragments[0]
                    .replace("<id>", "weld-bus")
                    .replace("<lang>", "ts")
                    .replace("<anchor>", "call-name+arg-position")
                    .replace("<n>", "1");
                let fx = Fixture::new();
                fx.write(
                    "package.json",
                    "{\n  \"name\": \"patternweld\",\n  \"version\": \"1.0.0\"\n}\n",
                );
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"ts\"\n\n[[pattern_pair]]\nid = \"weld-bus\"\n\n[pattern_pair.emit]\nlanguage = \"ts\"\nrole = \"publish\"\ncall = \"notify\"\narg = 0\n",
                );
                fx.write(
                    "src/a.ts",
                    "export function go() {\n  notify(\"thing.done\");\n}\n",
                );
                let declaring = fx.run(&["scan", "--output", "-"]);
                if declaring.status.code() != Some(0)
                    || !stderr(&declaring).contains(line.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: a declaring ts scan does not print the registered census line {line:?} — exit {:?}, stderr {:?}",
                        claim.id,
                        declaring.status.code(),
                        stderr(&declaring)
                    ));
                }
                let fx = Fixture::new();
                fx.write(
                    "package.json",
                    "{\n  \"name\": \"patternweld\",\n  \"version\": \"1.0.0\"\n}\n",
                );
                fx.write("src/a.ts", "export function go() {}\n");
                let quiet = fx.run(&["scan", "--output", "-"]);
                if stderr(&quiet).contains("literal sites:") || stderr(&quiet).contains("driver-gap:") {
                    errors.push(format!(
                        "claim `{}` drifted: a non-declaring run prints census or gap bytes (4.2 arm 1): {}",
                        claim.id,
                        stderr(&quiet)
                    ));
                }
            }
            "value-join-rows" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
                // Live probe (workplan events_value_join scenario 04): a
                // declaring rust fixture whose publish and subscribe sites
                // share one value prints the registered derived-row line
                // with the placeholders resolved, and a non-declaring run
                // prints no value-join bytes at all (4.2 arm 1).
                let line = claim.canonical_fragments[0]
                    .replace("<from>", "pubweld")
                    .replace("<to>", "subweld")
                    .replace("<event>", "thing.done");
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[workspace]\nmembers = [\"pubweld\", \"subweld\"]\nresolver = \"2\"\n",
                );
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"weld-bus\"\n\n[pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"notify\"\narg = 0\n\n[pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"attach\"\narg = 0\n",
                );
                fx.write(
                    "pubweld/Cargo.toml",
                    "[package]\nname = \"pubweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write(
                    "pubweld/src/lib.rs",
                    "pub fn go() {\n    notify(\"thing.done\");\n}\n\nfn notify(_event: &str) {}\n",
                );
                fx.write(
                    "subweld/Cargo.toml",
                    "[package]\nname = \"subweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write(
                    "subweld/src/lib.rs",
                    "pub fn init() {\n    attach(\"thing.done\");\n}\n\nfn attach(_event: &str) {}\n",
                );
                let declaring = fx.run(&["scan", "--output", "-"]);
                if declaring.status.code() != Some(0)
                    || !stderr(&declaring).contains(line.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: a declaring rust scan does not print the registered derived-row line {line:?} — exit {:?}, stderr {:?}",
                        claim.id,
                        declaring.status.code(),
                        stderr(&declaring)
                    ));
                }
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"joinweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub fn go() {}\n");
                let quiet = fx.run(&["scan", "--output", "-"]);
                if stderr(&quiet).contains("async-row derived")
                    || stderr(&quiet).contains("one-sided-leg")
                {
                    errors.push(format!(
                        "claim `{}` drifted: a non-declaring run prints value-join bytes (4.2 arm 1): {}",
                        claim.id,
                        stderr(&quiet)
                    ));
                }
            }
            "attribution-census-rows" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
                // Live probe (workplan events_value_join scenario 05): a
                // declaring fixture whose hand-declared row names the
                // declaring unit while the dispatch-side fact names the
                // publishing unit prints the registered census line with
                // the placeholders resolved, exactly once, exit 0; the
                // same tree with the row attributed to the dispatching
                // unit prints no attribution bytes (4.4a(b) both arms).
                let line = claim.canonical_fragments[0]
                    .replace("<from>", "declweld")
                    .replace("<to>", "subweld")
                    .replace("<event>", "pubweld::ThingDone")
                    .replace("<units>", "pubweld");
                let spec_text = |from: &str| {
                    format!(
                        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"pubweld\"\nmatches = {{ units = [\"pubweld\"] }}\n\n[[module]]\nname = \"subweld\"\nmatches = {{ units = [\"subweld\"] }}\n\n[[module]]\nname = \"declweld\"\nmatches = {{ units = [\"declweld\"] }}\n\n[[pattern_pair]]\nid = \"weld-bus\"\n\n[pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"notify\"\narg = 0\n\n[pattern_pair.subscribe]\nlanguage = \"rust\"\nrole = \"subscribe\"\ncall = \"listen\"\narg = 0\n\n[[async_depends]]\nfrom = \"{from}\"\nto = \"subweld\"\nevent = \"pubweld::ThingDone\"\n"
                    )
                };
                let workspace = |fx: &Fixture, spec_from: &str| {
                    fx.write(
                        "Cargo.toml",
                        "[workspace]\nmembers = [\"pubweld\", \"subweld\", \"declweld\"]\nresolver = \"2\"\n",
                    );
                    fx.write("architecture.spec.toml", spec_text(spec_from).as_str());
                    fx.write(
                        "pubweld/Cargo.toml",
                        "[package]\nname = \"pubweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                    );
                    fx.write(
                        "pubweld/src/lib.rs",
                        "pub struct ThingDone;\n\npub fn go() {\n    notify(\"pubweld::ThingDone\");\n}\n\nfn notify(_event: &str) {}\n",
                    );
                    fx.write(
                        "subweld/Cargo.toml",
                        "[package]\nname = \"subweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                    );
                    fx.write(
                        "subweld/src/lib.rs",
                        "pub fn init() {\n    listen(\"pubweld::ThingDone\");\n}\n\nfn listen(_event: &str) {}\n",
                    );
                    fx.write(
                        "declweld/Cargo.toml",
                        "[package]\nname = \"declweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                    );
                    fx.write("declweld/src/lib.rs", "pub fn wiring() {}\n");
                };
                let fx = Fixture::new();
                workspace(&fx, "declweld");
                let mismatching = fx.run(&["verify", "."]);
                let hits = stderr(&mismatching)
                    .lines()
                    .filter(|line| line.contains("ambiguous-publish-attribution"))
                    .count();
                if mismatching.status.code() != Some(0)
                    || hits != 1
                    || !stderr(&mismatching).contains(line.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: a mismatching declared row does not print the registered census line {line:?} exactly once — exit {:?}, stderr {:?}",
                        claim.id,
                        mismatching.status.code(),
                        stderr(&mismatching)
                    ));
                }
                let fx = Fixture::new();
                workspace(&fx, "pubweld");
                let attributed = fx.run(&["verify", "."]);
                if stderr(&attributed).contains("ambiguous-publish-attribution") {
                    errors.push(format!(
                        "claim `{}` drifted: a row whose declaring side equals the dispatching side prints attribution bytes (4.4a(b) zero-row arm): {}",
                        claim.id,
                        stderr(&attributed)
                    ));
                }
            }
            // Registered by the dispatch-blinded projection (workplan
            // events_value_join scenario 06, F-W4): the row reads the
            // shipped dispatch rows plus the literal-site rows and prints
            // ONE census line per blinded zone — report-only, never new
            // scan machinery, never a verdict.
            "dispatch-blinded-literals" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
                // Live probe: a declaring fixture whose dual-marker zone
                // dispatches a declared language OUT prints the registered
                // line with the placeholders resolved; the same tree with
                // no pattern pair prints no dispatch-blinded bytes at all.
                let line = claim.canonical_fragments[0]
                    .replace("<path>", "weldzone")
                    .replace("<set>", "\"ts\"")
                    .replace("<driver>", "rust")
                    .replace("<n>", "0");
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"blindweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub fn api() {}\n");
                fx.write(
                    "weldzone/Cargo.toml",
                    "[package]\nname = \"blindweld-zone\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write(
                    "weldzone/package.json",
                    "{\"name\": \"blindweld-zone\", \"version\": \"0.1.0\"}\n",
                );
                fx.write(
                    "weldzone/events.ts",
                    "export function init() {\n    attach(\"weld.done\");\n}\n\nfunction attach(_event: string) {}\n",
                );
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\nexcluded_sources = [ { path = \"weldzone\", reason = \"the zone ships its ts side separately\" } ]\n\n[[pattern_pair]]\nid = \"weld-blind\"\n\n[pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\ncall = \"notify\"\narg = 0\n\n[pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\ncall = \"attach\"\narg = 0\n",
                );
                let declaring = fx.run(&["scan", "--output", "-"]);
                if declaring.status.code() != Some(0)
                    || !stderr(&declaring).contains(line.as_str())
                {
                    errors.push(format!(
                        "claim `{}` drifted: a declaring dual-marker scan does not print the registered dispatch-blinded line {line:?} — exit {:?}, stderr {:?}",
                        claim.id,
                        declaring.status.code(),
                        stderr(&declaring)
                    ));
                }
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\nexcluded_sources = [ { path = \"weldzone\", reason = \"the zone ships its ts side separately\" } ]\n",
                );
                let quiet = fx.run(&["scan", "--output", "-"]);
                if stderr(&quiet).contains("dispatch-blinded-literals") {
                    errors.push(format!(
                        "claim `{}` drifted: a non-declaring run prints dispatch-blinded bytes (4.2 arm 1): {}",
                        claim.id,
                        stderr(&quiet)
                    ));
                }
            }
            "async-depends-inert-notes" => {
                // Live probe: a rust run with a NON-EMPTY async_depends table
                // and no declared-type census facts prints both notes
                // verbatim, exit 0 — rust derives nothing (capability row
                // not-emitted) and the fixture's lib declares no public item.
                let canonicals: Vec<&str> = claim.canonical_fragments.iter().map(String::as_str).collect();
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"asyncweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "fn hidden() {}\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"asyncweld\"\nmatches = { units = [\"asyncweld\"] }\n\n[[async_depends]]\nfrom = \"asyncweld\"\nto = \"asyncweld\"\nevent = \"Ghost.Events.OrderPlaced\"\n",
                );
                let run = fx.run(&["verify"]);
                if run.status.code() != Some(0) {
                    errors.push(format!(
                        "claim `{}` drifted: a rust run with an async_depends table did not exit 0: {}",
                        claim.id,
                        stdout(&run)
                    ));
                }
                for canonical in canonicals {
                    if !stdout(&run).contains(canonical) {
                        errors.push(format!(
                            "claim `{}` drifted: a rust verify run does not emit the registered inert note verbatim: {}",
                            claim.id,
                            stdout(&run)
                        ));
                    }
                }
            }
            "async-census-section" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
                // Live probe, both sides: a rust run whose declared table
                // resolves one pair renders the section header on the report
                // surface; a run with no async table renders no section
                // bytes at all (D8 — absence is the contract).
                let spec_with = "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"censusweld\"\nmatches = { units = [\"censusweld\"] }\n\n[[async_depends]]\nfrom = \"censusweld\"\nto = \"censusweld\"\nevent = \"Ghost.Events.OrderPlaced\"\n";
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"censusweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub fn api() {}\n");
                fx.write("architecture.spec.toml", spec_with);
                let declaring = fx.run(&["report"]);
                if !stdout(&declaring).contains("Async census") {
                    errors.push(format!(
                        "claim `{}` drifted: a declaring rust report does not render the section header: {}",
                        claim.id,
                        stdout(&declaring)
                    ));
                }
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"censusweld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub fn api() {}\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"censusweld\"\nmatches = { units = [\"censusweld\"] }\n",
                );
                let bare = fx.run(&["report"]);
                if stdout(&bare).contains("Async census") {
                    errors.push(format!(
                        "claim `{}` drifted: a non-declaring rust report renders async census bytes: {}",
                        claim.id,
                        stdout(&bare)
                    ));
                }
            }
            "ts-vacuity-carriers" => {
                // The four carriers are emitted by the ts legs of scan,
                // inspect and verify on EMPTY evidence sets — the weld runs
                // one probe per carrier and requires every registered
                // carrier line in the joined streams.
                let mut emitted = String::new();
                let fx = Fixture::new();
                fx.write("a.ts", "export const a = 1;\n");
                let scan = fx.run(&["scan", "."]);
                emitted.push_str(&stdout(&scan));
                emitted.push('\n');
                emitted.push_str(&stderr(&scan));
                let fx = Fixture::new();
                fx.write(
                    "package.json",
                    "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
                );
                fx.write("src/a.ts", "export const a = 1;\n");
                fx.write("src/b.ts", "export const b = 2;\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"src/a.ts\"] }\n\n[module.allowed]\nforbidden = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"src/b.ts\"] }\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"] }\n",
                );
                let inspect = fx.run(&["inspect"]);
                emitted.push_str(&stdout(&inspect));
                emitted.push('\n');
                emitted.push_str(&stderr(&inspect));
                let verify = fx.run(&["verify", "."]);
                emitted.push_str(&stdout(&verify));
                emitted.push('\n');
                emitted.push_str(&stderr(&verify));
                let fx = Fixture::new();
                fx.write(
                    "package.json",
                    "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
                );
                fx.write("index.ts", "export { secret } from \"./src/internal\";\n");
                fx.write("src/internal.ts", "export const secret = 1;\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"demo-ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"index.ts\", \"src/internal.ts\"] }\n",
                );
                let verify = fx.run(&["verify", "."]);
                emitted.push_str(&stdout(&verify));
                emitted.push('\n');
                emitted.push_str(&stderr(&verify));
                for fragment in &claim.canonical_fragments {
                    if !emitted.contains(fragment.as_str()) {
                        errors.push(format!(
                            "claim `{}` drifted: the ts vacuity probes do not render the registered carrier {fragment:?}",
                            claim.id
                        ));
                    }
                }
            }
            "npm-channel-install" => {
                // The welded statement surface: the agents doc states the
                // npm-channel install sentence beside the release-asset
                // guidance; the machine-readable half of the contract lives
                // in the dedicated guard tests/npm_channel.rs.
                let canonical = claim.canonical.clone().expect("npm-channel canonical");
                let needle = flatten(&canonical).to_lowercase();
                if !flatten(&load("docs/archspec/agents.md"))
                    .to_lowercase()
                    .contains(&needle)
                {
                    errors.push(format!(
                        "claim `{}` drifted: the agents doc no longer states the registered npm-channel sentence",
                        claim.id
                    ));
                }
            }
            "clusters-help-flags" => {
                contains_canonical(&mut errors, claim, "inspect --help");
                contains_canonical(&mut errors, claim, "help inspect");
            }
            "verify-help-json-flag" => {
                contains_canonical(&mut errors, claim, "verify --help");
                contains_canonical(&mut errors, claim, "help verify");
            }
            "explain-renderer" => {
                contains_canonical(&mut errors, claim, "explain --help");
                contains_canonical(&mut errors, claim, "help explain");
            }
            "verdict-environment-declared" | "verify-only-class-filter" => {
                contains_canonical(&mut errors, claim, "verify --help");
                contains_canonical(&mut errors, claim, "help verify");
            }
            "verify-help-seed-flag" => {
                contains_canonical(&mut errors, claim, "verify --help");
                contains_canonical(&mut errors, claim, "help verify");
            }
            "verify-help-repro-flag" => {
                contains_canonical(&mut errors, claim, "verify --help");
                contains_canonical(&mut errors, claim, "help verify");
            }
            "cycle-repro-lines" => {
                // Live probe, not a prose echo: the flagged run of the canonical
                // TDZ pair must render every registered repro line, and the
                // emitter source must still carry the expectation template.
                let fx = Fixture::new();
                fx.write(
                    "package.json",
                    "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
                );
                fx.write("src/a.ts", "import { b } from \"./b\";\nexport const a = b;\n");
                fx.write("src/b.ts", "import { a } from \"./a\";\nexport const b = a;\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
                );
                let emitted = stdout(&fx.run(&["verify", ".", "--emit-repro"]));
                for fragment in &claim.canonical_fragments {
                    if !emitted.contains(fragment.as_str()) {
                        errors.push(format!(
                            "claim `{}` drifted: the flagged verify probe does not render the registered repro line {fragment:?}",
                            claim.id
                        ));
                    }
                }
                let emitter_src = load("src/archspec/verify/compare/cycles.rs");
                if !emitter_src.contains("TDZ ReferenceError in") {
                    errors.push(format!(
                        "claim `{}` drifted: the repro emitter source no longer carries the expectation template",
                        claim.id
                    ));
                }
            }
            "cycle-repro-language-skip" => {
                // Live probe on a non-ts runtime ring: the named skip row must
                // render with the language spelled into the registered shape.
                let fx = Fixture::new();
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"python\"\n\n[[module]]\nname = \"app\"\nmatches = { modules = [\"**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
                );
                fx.write("a.py", "from b import b\nvalue = b\n");
                fx.write("b.py", "from a import a\nvalue = a\n");
                let emitted = stdout(&fx.run(&["verify", ".", "--emit-repro"]));
                let skip = "repro: import-order one-liners are implemented for the ts driver — no runnable form for python";
                if !emitted.contains(skip) {
                    errors.push(format!(
                        "claim `{}` drifted: the flagged python probe does not render the registered skip row: {emitted}",
                        claim.id
                    ));
                }
                let emitter_src = load("src/archspec/verify/compare/cycles.rs");
                if !emitter_src.contains("no runnable form for ") {
                    errors.push(format!(
                        "claim `{}` drifted: the repro emitter source no longer carries the skip row",
                        claim.id
                    ));
                }
            }
            "exempt-ratchet-diagnostics" => {
                contains_canonical(&mut errors, claim, "help diagnostics");
            }
            "clusters-structure-verdicts" => {
                // The three live classes: one probe per reachable verdict.
                // The trivial class needs edges>0 with all-singletons; the
                // greedy path merges every inspect-reachable graph's first
                // positive pair, so that one label is welded to the labelling
                // source (structure_label, the single labelling surface).
                let mut emitted = String::new();
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"wide\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub mod a;\npub mod b;\npub mod c;\n");
                fx.write("src/a.rs", "pub const A: u32 = 1;\n");
                fx.write("src/b.rs", "pub const B: u32 = 2;\n");
                fx.write("src/c.rs", "pub const C: u32 = 3;\n");
                emitted.push_str(&stdout(&fx.run(&["inspect", "tree", ".", "--clusters"])));
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"pair\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub mod a;\npub mod b;\n");
                fx.write("src/a.rs", "pub const A: u32 = 1;\n");
                fx.write("src/b.rs", "use crate::a;\npub const B: u32 = a::A;\n");
                emitted.push_str(&stdout(&fx.run(&["inspect", "tree", ".", "--clusters"])));
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"twopair\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub mod a;\npub mod b;\npub mod c;\npub mod d;\n");
                fx.write("src/a.rs", "pub const A: u32 = 1;\n");
                fx.write("src/b.rs", "use crate::a;\npub const B: u32 = a::A;\n");
                fx.write("src/c.rs", "pub const C: u32 = 3;\n");
                fx.write("src/d.rs", "use crate::c;\npub const D: u32 = c::C;\n");
                let twopair = stdout(&fx.run(&["inspect", "tree", ".", "--clusters"]));
                emitted.push_str(&twopair);
                if !twopair.contains("structure: 2 clusters") {
                    errors.push(format!(
                        "claim `{}` drifted: the two-pair probe does not render the count form of the structure line: {twopair}",
                        claim.id
                    ));
                }
                let label_src = load("src/archspec/inspect/clusters.rs");
                for fragment in &claim.canonical_fragments {
                    let body = fragment.strip_prefix("structure: ").unwrap_or(fragment);
                    if !emitted.contains(fragment.as_str())
                        && !label_src.contains(&format!("\"{body}\""))
                    {
                        errors.push(format!(
                            "claim `{}` drifted: neither the live verdict probes nor the labelling source render the registered label {fragment:?}",
                            claim.id
                        ));
                    }
                }
            }
            "clusters-cap-skip" => {
                // The caps are deliberately large, so no cheap live probe can
                // cross them; the weld pins the emit strings and the cap
                // constants in the labelling source plus the concrete skip
                // lines the dedicated guard asserts from live runs.
                let label_src = load("src/archspec/inspect/clusters.rs");
                for needle in [
                    "skipped (nodes=",
                    "skipped (edges=",
                    "CAP_NODES: usize = 10_000",
                    "CAP_EDGES: u64 = 130_000",
                ] {
                    if !label_src.contains(needle) {
                        errors.push(format!(
                            "claim `{}` drifted: the clustering source no longer states {needle:?}",
                            claim.id
                        ));
                    }
                }
                let guard = load("tests/inspect_clusters.rs");
                for needle in [
                    "clusters: skipped (nodes=10001 > cap 10000)",
                    "clusters: skipped (edges=260610 > cap 130000)",
                ] {
                    if !guard.contains(needle) {
                        errors.push(format!(
                            "claim `{}` drifted: the dedicated guard no longer pins the live skip line {needle:?}",
                            claim.id
                        ));
                    }
                }
            }
            "spec-schema-fail-closed-shape" => {
                // Live probes, not prose echoes: the emitted schema must
                // close every table shape it names and carry the
                // excluded_sources block, and the loader must refuse a key
                // outside the shape with the registered refusal string.
                let fx = Fixture::new();
                let schema = stdout(&fx.run(&["spec", "--schema"]));
                if !schema.contains("\"additionalProperties\": false") {
                    errors.push(format!(
                        "claim `{}` drifted: the emitted schema no longer states `additionalProperties: false`",
                        claim.id
                    ));
                }
                if !schema.contains("\"excluded_sources\"") {
                    errors.push(format!(
                        "claim `{}` drifted: the emitted schema no longer names project.excluded_sources",
                        claim.id
                    ));
                }
                let fx = Fixture::new();
                fx.write(
                    "Cargo.toml",
                    "[package]\nname = \"weld\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                );
                fx.write("src/lib.rs", "pub fn f() {}\n");
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"rust\"\nmode = \"report-only\"\n",
                );
                let probe = fx.run(&["verify", "."]);
                let verdict = format!("{}{}", stdout(&probe), stderr(&probe));
                if probe.status.code() == Some(0) || !verdict.contains("unknown key(s) \"mode\"") {
                    errors.push(format!(
                        "claim `{}` drifted: the loader no longer refuses an undefined key naming the key and its location: {verdict}",
                        claim.id
                    ));
                }
                // The probes exercise the one shared table; the weld pins its
                // name at the single source both halves read.
                let shape_src = load("src/archspec/spec.rs");
                if !shape_src.contains("SPEC_KEY_SHAPES") {
                    errors.push(format!(
                        "claim `{}` drifted: the loader source no longer carries the shared SPEC_KEY_SHAPES table",
                        claim.id
                    ));
                }
            }
            "verify-help-strict-flag" => {
                contains_canonical(&mut errors, claim, "verify --help");
                contains_canonical(&mut errors, claim, "help verify");
                contains_canonical(&mut errors, claim, "help workflow");
            }
            "verify-verdict-headline" => {
                // Live probe, not a prose echo: a warnings-only verify run
                // exits 0 with the registered passing headline, and the same
                // tree under `--strict` keeps the shipped mismatch bytes
                // (GitHub issue #54).
                let fx = Fixture::new();
                fx.write("go.mod", "module example.com/demo\ngo 1.21\n");
                for (dir, package, imports) in [
                    ("a", "a", &["example.com/demo/shell"][..]),
                    ("shell", "shell", &["example.com/demo/store"][..]),
                    ("store", "store", &[] as &[&str]),
                ] {
                    let mut src = format!("package {package}\n");
                    if !imports.is_empty() {
                        src.push_str("\nimport (\n");
                        for import in imports {
                            src.push_str(&format!("    {import:?}\n"));
                        }
                        src.push_str(")\n");
                    }
                    src.push('\n');
                    fx.write(&format!("{dir}/{package}.go"), &src);
                }
                fx.write(
                    "architecture.spec.toml",
                    "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\n[module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n[[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\n[module.allowed]\ndepend_on = [\"store\"]\n\n[[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
                );
                let passing = fx.run(&["verify"]);
                let out = stdout(&passing);
                if passing.status.code() != Some(0)
                    || !out.contains("ok: architecture.spec.toml passes with 1 warning")
                    || out.contains("does not match source model")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the warnings-only run no longer exits 0 with the registered passing headline:\n{out}",
                        claim.id
                    ));
                }
                let failing = fx.run(&["verify", "--strict"]);
                let out = stdout(&failing);
                if failing.status.code() == Some(0)
                    || !out.contains("architecture.spec.toml does not match source model")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the promoted run no longer prints the registered mismatch headline:\n{out}",
                        claim.id
                    ));
                }
            }
            "layer-selector-refusal" => {
                // Live probes, not prose: both verbs refuse an unknown layer
                // with the registered canonical bytes at exit 64 (workplan
                // visibility_phase2 scenario 05). The flag parses before any
                // model load, so an empty fixture reaches the refusal.
                let canonical = claim.canonical.clone().expect("canonical");
                let fx = Fixture::new();
                for args in [
                    ["diagram", "--layer", "sideways"].as_slice(),
                    ["depgraph", "modules", "--layer", "sideways"].as_slice(),
                ] {
                    let refused = fx.run(args);
                    if refused.status.code() != Some(64)
                        || !stderr(&refused).contains(canonical.as_str())
                    {
                        errors.push(format!(
                            "claim `{}` drifted: `{}` does not refuse with the registered canonical bytes at exit 64 — exit {:?}, stderr {:?}",
                            claim.id,
                            args.join(" "),
                            refused.status.code(),
                            stderr(&refused)
                        ));
                    }
                }
            }
            "layer-ceiling-rationale" => {
                contains_canonical(&mut errors, claim, "diagram --help");
            }
            "mermaid-ceiling-refusal" => {
                // The live refusal bytes at the emit seam (sweep HIGH-1
                // cure): a 501-edge synthetic artefact refuses at exit 64
                // with the registered template's head and edge count, and
                // emits no body — the failure never waits for a mermaid
                // host to stay silent.
                let canonical = claim.canonical.clone().expect("canonical");
                let chunks: Vec<&str> = canonical.split("<path>").collect();
                let fx = Fixture::new();
                let units: Vec<String> = (0..=501)
                    .map(|i| format!("  {{ \"name\": \"u{i:03}\", \"kind\": \"lib\", \"path\": \"u{i:03}\" }}"))
                    .collect();
                let edges: Vec<String> = (0..501)
                    .map(|i| format!("  {{ \"from\": \"u{i:03}\", \"to\": \"u{:03}\" }}", i + 1))
                    .collect();
                fx.write(
                    "model.json",
                    &format!(
                        "{{\n  \"schema_version\": 1,\n  \"language\": \"rust\",\n  \"units\": [\n{}\n  ],\n  \"edges\": [\n{}\n  ],\n  \"external\": [],\n  \"async_edges\": []\n}}\n",
                        units.join(",\n"),
                        edges.join(",\n"),
                    ),
                );
                let refused = fx.run(&["diagram", "--source", "scan", "model.json"]);
                let err = stderr(&refused);
                let head_with_count = format!("{}501{}", chunks[0], chunks[1]);
                if refused.status.code() != Some(64)
                    || !err.contains(head_with_count.as_str())
                    || !err.contains(chunks[2])
                    || stdout(&refused).contains("graph")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the live over-ceiling render does not refuse with the registered template bytes at exit 64 — exit {:?}, stderr {:?}",
                        claim.id,
                        refused.status.code(),
                        err
                    ));
                }
            }
            "viewer-single-file-law" | "viewer-channels-limitation" | "viewer-interaction-controls" => {
                contains_canonical(&mut errors, claim, "diagram --help");
                contains_canonical(&mut errors, claim, "help diagram");
            }
            "level-selector-refusal"
            | "level-dir-floor-refusal"
            | "level-source-refusal"
            | "level-tiers-refusal" => {
                // Live probes, not prose: the flag parses before any model
                // load (the layer refusals' precedent), so an empty fixture
                // reaches every `--level` usage refusal at exit 64 with the
                // registered canonical bytes (workplan level_views 01).
                let canonical = claim.canonical.clone().expect("canonical");
                let fx = Fixture::new();
                let probes: &[&[&str]] = match claim.id.as_str() {
                    "level-selector-refusal" => &[["diagram", "--level", "sideways"].as_slice()],
                    "level-dir-floor-refusal" => &[["diagram", "--level", "dir:0"].as_slice()],
                    "level-source-refusal" => &[["diagram", "--level", "module"].as_slice()],
                    _ => &[[
                        "diagram",
                        "--source",
                        "scan",
                        "model.json",
                        "--tiers",
                        "units",
                        "--level",
                        "unit",
                    ]
                    .as_slice()],
                };
                for args in probes {
                    let refused = fx.run(args);
                    if refused.status.code() != Some(64)
                        || !stderr(&refused).contains(canonical.as_str())
                    {
                        errors.push(format!(
                            "claim `{}` drifted: `{}` does not refuse with the registered canonical bytes at exit 64 — exit {:?}, stderr {:?}",
                            claim.id,
                            args.join(" "),
                            refused.status.code(),
                            stderr(&refused)
                        ));
                    }
                }
            }
            "level-descriptive-note" | "level-accounting-exact" => {
                let fx = Fixture::new();
                level_weld_artefact(&fx);
                let run = fx.run(&["diagram", "--source", "scan", "model.json", "--level", "module"]);
                let err = stderr(&run);
                if run.status.code() != Some(0) {
                    errors.push(format!(
                        "claim `{}` drifted: the level render exits {:?}: {}",
                        claim.id,
                        run.status.code(),
                        err
                    ));
                } else if claim.id == "level-descriptive-note" {
                    let canonical = claim.canonical.clone().expect("canonical");
                    if !err.contains(canonical.as_str()) {
                        errors.push(format!(
                            "claim `{}` drifted: the level render does not print the registered note: {err}",
                            claim.id
                        ));
                    }
                } else {
                    // The weld instantiates the registered template with the
                    // fixture's fixed counts and re-checks the arithmetic:
                    // 3 drawn + 1 self = 4 raw rows at level module.
                    let canonical = claim.canonical.clone().expect("canonical");
                    let chunks: Vec<&str> = canonical.split("<path>").collect();
                    let row = format!(
                        "{}3{}1{}4{}module",
                        chunks[0], chunks[1], chunks[2], chunks[3]
                    );
                    if !err.contains(row.as_str()) {
                        errors.push(format!(
                            "claim `{}` drifted: the ledger row does not instantiate the registered template — expected {row:?} in: {err}",
                            claim.id
                        ));
                    }
                    if !err.contains("self rows: gateway") {
                        errors.push(format!(
                            "claim `{}` drifted: the accounting block omits the self tally: {err}",
                            claim.id
                        ));
                    }
                    if !stdout(&run).contains("|3 rows|") {
                        errors.push(format!(
                            "claim `{}` drifted: the aggregated edge does not draw its raw-row count: {}",
                            claim.id,
                            stdout(&run)
                        ));
                    }
                }
            }
            "level-empty-model" => {
                let fx = Fixture::new();
                fx.write(
                    "model.json",
                    r#"{"schema_version":1,"language":"rust","units":[{"name":"auth","kind":"crate","path":"crates/auth"}],"edges":[]}"#,
                );
                let run = fx.run(&["diagram", "--source", "scan", "model.json", "--level", "unit"]);
                let err = stderr(&run);
                let canonical = claim.canonical.clone().expect("canonical");
                let head = canonical.split("<path>").next().expect("the row head");
                if run.status.code() != Some(0)
                    || !err.contains(&format!("{head}unit"))
                    || err.contains("accounting exact:")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the zero-row level render does not print the named empty-model row in place of the ledger row — exit {:?}, stderr {:?}",
                        claim.id,
                        run.status.code(),
                        err
                    ));
                }
            }
            "level-completeness-restatement" => {
                let fx = Fixture::new();
                fx.write(
                    "model.json",
                    r#"{"schema_version":1,"language":"rust","units":[{"name":"auth","kind":"crate","path":"crates/auth"}],"edges":[],"module_edges":[{"unit":"auth","from":"crates/auth/login","to":"crates/auth/store","symbols":[]}],"unresolvable_imports":[{"file":"crates/auth/login.rs","specifier":"ghost::x","line":"3"}]}"#,
                );
                let run = fx.run(&["diagram", "--source", "scan", "model.json", "--level", "dir:1"]);
                let err = stderr(&run);
                if run.status.code() != Some(0)
                    || !err.contains("completeness restated: unresolvable imports census:")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the level render does not restate the completeness census row — exit {:?}, stderr {:?}",
                        claim.id,
                        run.status.code(),
                        err
                    ));
                }
            }
            "level-unit-delta-model-only" | "level-unit-delta-projection-only" => {
                // Live probe plus the help-parity row (workplan level_views
                // scenario 03): the delta fixture renders rc=0 with both
                // census classes named, and the `--level` help row states
                // the class head verbatim.
                let fx = Fixture::new();
                fx.write(
                    "model.json",
                    r#"{"schema_version":1,"language":"rust","units":[{"name":"workspace","kind":"lib","path":"."},{"name":"alpha","kind":"lib","path":"src/alpha"},{"name":"beta","kind":"lib","path":"src/beta"},{"name":"gamma","kind":"lib","path":"src/gamma"}],"edges":[{"from":"alpha","to":"beta"},{"from":"beta","to":"alpha"},{"from":"beta","to":"gamma"}],"module_edges":[{"unit":"alpha","from":"alpha::x","to":"beta::store","symbols":[]},{"unit":"beta","from":"beta::q","to":"alpha::y","symbols":[]},{"unit":"beta","from":"beta::m","to":"beta::n","symbols":[]},{"unit":"workspace","from":"tools/x.rs","to":"beta::store","symbols":[]}]}"#,
                );
                let run = fx.run(&["diagram", "--source", "scan", "model.json", "--level", "unit"]);
                let err = stderr(&run);
                if run.status.code() != Some(0) {
                    errors.push(format!(
                        "claim `{}` drifted: the unit-level render exits {:?}: {err}",
                        claim.id,
                        run.status.code()
                    ));
                }
                let canonical = claim.canonical.clone().expect("canonical");
                let head = canonical.split("<path>").next().expect("the row head");
                let row = match claim.id.as_str() {
                    "level-unit-delta-model-only" => format!("{head}1 — beta→gamma"),
                    _ => format!("{head}1 — workspace→beta"),
                };
                if !err.contains(row.as_str()) {
                    errors.push(format!(
                        "claim `{}` drifted: the census does not instantiate the registered template — expected {row:?} in: {err}",
                        claim.id
                    ));
                }
                if stdout(&run).contains("unit delta") {
                    errors.push(format!(
                        "claim `{}` drifted: a census row entered the render body: {}",
                        claim.id,
                        stdout(&run)
                    ));
                }
                let help = flatten(&rendered("diagram --help")).to_lowercase();
                if !help.contains(&flatten(head).to_lowercase()) {
                    errors.push(format!(
                        "claim `{}` drifted: `archspec diagram --help` no longer states the census class head {head:?}",
                        claim.id
                    ));
                }
            }
            // Over-ceiling level fixture for the fit-set refusal welds
            // (workplan level_views scenario 02): 501 distinct cross-directory
            // pairs — the unit and dir:1 folds aggregate them to one drawn
            // edge each (FIT), the dir:2 fold draws 501 edge rows (the first
            // OVER fold), and every finer fold goes unmeasured.
            "level-fits-listing" => {
                let fx = Fixture::new();
                let edges: Vec<String> = (0..501)
                    .map(|i| {
                        format!(
                            "{{\"unit\":\"alpha\",\"from\":\"alpha/d{i}/f.rs\",\"to\":\"beta/e{i}/f.rs\",\"symbols\":[]}}"
                        )
                    })
                    .collect();
                fx.write(
                    "model.json",
                    &format!(
                        "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"alpha\",\"kind\":\"lib\",\"path\":\"alpha\"}},{{\"name\":\"beta\",\"kind\":\"lib\",\"path\":\"beta\"}}],\"edges\":[],\"module_edges\":[{}]}}",
                        edges.join(",")
                    ),
                );
                let run =
                    fx.run(&["diagram", "--source", "scan", "model.json", "--level", "file"]);
                let err = stderr(&run);
                if run.status.code() != Some(64) || !err.contains("fits: unit (0% self)") {
                    errors.push(format!(
                        "claim `{}` drifted: the over-ceiling level refusal does not name the fits — exit {:?}, stderr {:?}",
                        claim.id,
                        run.status.code(),
                        err
                    ));
                }
            }
            "level-empty-fits-row" => {
                // Zero-cross-at-every-fold fixture: 3000 self rows whose
                // file-level node rows cross the char ceiling while every
                // fold keeps cross-edges at 0 — the named empty-fits row,
                // never silence (B2/M10).
                let fx = Fixture::new();
                let edges: Vec<String> = (0..3000)
                    .map(|i| {
                        format!(
                            "{{\"unit\":\"src\",\"from\":\"src/s/f{i}.rs\",\"to\":\"src/s/f{i}.rs\",\"symbols\":[]}}"
                        )
                    })
                    .collect();
                fx.write(
                    "model.json",
                    &format!(
                        "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"src\",\"kind\":\"lib\",\"path\":\"src\"}}],\"edges\":[],\"module_edges\":[{}]}}",
                        edges.join(",")
                    ),
                );
                let run =
                    fx.run(&["diagram", "--source", "scan", "model.json", "--level", "file"]);
                let err = stderr(&run);
                if run.status.code() != Some(64)
                    || !err.contains("fits: none — no fold fits at these ceilings")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the zero-cross over-ceiling refusal does not print the named empty-fits row — exit {:?}, stderr {:?}",
                        claim.id,
                        run.status.code(),
                        err
                    ));
                }
            }
            "level-narrowing-grammar" => {
                let fx = Fixture::new();
                let edges: Vec<String> = (0..501)
                    .map(|i| {
                        format!(
                            "{{\"unit\":\"alpha\",\"from\":\"alpha/d{i}/f.rs\",\"to\":\"beta/e{i}/f.rs\",\"symbols\":[]}}"
                        )
                    })
                    .collect();
                fx.write(
                    "model.json",
                    &format!(
                        "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"alpha\",\"kind\":\"lib\",\"path\":\"alpha\"}},{{\"name\":\"beta\",\"kind\":\"lib\",\"path\":\"beta\"}}],\"edges\":[],\"module_edges\":[{}]}}",
                        edges.join(",")
                    ),
                );
                let run =
                    fx.run(&["diagram", "--source", "scan", "model.json", "--level", "file"]);
                let err = stderr(&run);
                if run.status.code() != Some(64)
                    || !err.contains(
                        "narrow with: archspec scan <bucket-path> --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]",
                    )
                {
                    errors.push(format!(
                        "claim `{}` drifted: the over-ceiling level refusal does not name the two-step narrowing — exit {:?}, stderr {:?}",
                        claim.id,
                        run.status.code(),
                        err
                    ));
                }
            }
            // Level-viewer pane welds (workplan level_views scenario 04):
            // the recursive panes of the `--format viewer` level document.
            "viewer-byte-ceiling-refusal" => {
                // The pane-explosion mass over the viewer emitted-byte
                // ceiling: the emit ships nothing, refuses in the usage
                // class, and names the largest scope that fits.
                let run = ceiling_fixture.run(&[
                    "diagram",
                    "--source",
                    "scan",
                    "model.json",
                    "--level",
                    "module",
                    "--format",
                    "viewer",
                ]);
                let err = stderr(&run);
                if run.status.code() != Some(64)
                    || !err.contains("exceeds the emitted-byte ceiling of")
                    || !err.contains("largest scope that fits")
                    || !err.contains("narrow with: archspec scan <bucket-path> --output <artefact>")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the over-ceiling viewer emit does not refuse with the fitting scope — exit {:?}, stderr {err:?}",
                        claim.id,
                        run.status.code()
                    ));
                }
            }
            "level-pane-refusal" => {
                // Bucket `b` is over ceiling at its FINEST relative
                // granularity (600 distinct `dir:1` pairs over the root
                // `b`): the pane speaks the named refusal row with the
                // real bucket path substituted, and restates the census.
                let fx = Fixture::new();
                let edges: Vec<String> = (0..600)
                    .map(|i| {
                        format!(
                            "{{\"unit\":\"model\",\"from\":\"b/g{i}/x{i}.rs\",\"to\":\"b/h{i}/y{i}.rs\",\"symbols\":[]}}"
                        )
                    })
                    .collect();
                fx.write(
                    "model.json",
                    &format!(
                        "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"beta\",\"kind\":\"lib\",\"path\":\"b\"}}],\"edges\":[],\"module_edges\":[{}],\"unresolvable_imports\":[{{\"file\":\"b/g1/x1.rs\",\"specifier\":\"ghost-one\",\"line\":\"1\"}},{{\"file\":\"b/g2/x2.rs\",\"specifier\":\"ghost-two\",\"line\":\"1\"}}]}}",
                        edges.join(",")
                    ),
                );
                let run = fx.run(&[
                    "diagram",
                    "--source",
                    "scan",
                    "model.json",
                    "--level",
                    "dir:1",
                    "--format",
                    "viewer",
                ]);
                let out = stdout(&run);
                let row = "pane over ceiling at dir:1 — narrow: archspec scan b --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]"
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                if run.status.code() != Some(0)
                    || !out.contains(&row)
                    || !out.contains("completeness restated: unresolvable imports census:")
                {
                    errors.push(format!(
                        "claim `{}` drifted: the refusal pane does not speak the named row with the census restated — exit {:?}, stdout {out:?}",
                        claim.id,
                        run.status.code()
                    ));
                }
            }
            "level-document-total" => {
                // Two panes plus one cross-bucket row: the document states
                // the total row exactly once and its arithmetic holds.
                let fx = Fixture::new();
                fx.write(
                    "model.json",
                    "{\"schema_version\":1,\"language\":\"rust\",\"units\":[{\"name\":\"charlie\",\"kind\":\"lib\",\"path\":\"c\"},{\"name\":\"delta\",\"kind\":\"lib\",\"path\":\"d\"}],\"edges\":[],\"module_edges\":[{\"unit\":\"model\",\"from\":\"c/x/a/mod.rs\",\"to\":\"c/x/b/util.rs\",\"symbols\":[]},{\"unit\":\"model\",\"from\":\"d/x/a/mod.rs\",\"to\":\"d/x/b/util.rs\",\"symbols\":[]},{\"unit\":\"model\",\"from\":\"c/x/a/mod.rs\",\"to\":\"d/x/a/mod.rs\",\"symbols\":[]}]}",
                );
                let run = fx.run(&[
                    "diagram",
                    "--source",
                    "scan",
                    "model.json",
                    "--level",
                    "dir:1",
                    "--format",
                    "viewer",
                ]);
                let out = stdout(&run);
                let row = out
                    .lines()
                    .find(|line| line.contains("document total: "))
                    .unwrap_or("");
                let numbers: Vec<usize> = row
                    .split_whitespace()
                    .filter_map(|token| token.parse::<usize>().ok())
                    .collect();
                if run.status.code() != Some(0)
                    || out.matches("document total: ").count() != 1
                    || numbers.len() != 3
                    || numbers[0] + numbers[1] != numbers[2]
                {
                    errors.push(format!(
                        "claim `{}` drifted: the level-viewer document does not state the total row exactly once with its arithmetic holding — exit {:?}, row {row:?}",
                        claim.id,
                        run.status.code()
                    ));
                }
            }
            other => errors.push(format!(
                "claim `{other}` has no registry-to-surface weld — every registered claim must be compared against what its owner renders or emits"
            )),
        }
        if let Some(guard) = &claim.dedicated_guard {
            let text = std::fs::read_to_string(format!("{}/{}", manifest(), guard));
            if text.is_err() {
                errors.push(format!(
                    "claim `{}` names dedicated guard {guard:?} which does not exist",
                    claim.id
                ));
            }
        }
        for pin in &claim.guard_pins {
            let path = format!("{}/{}", manifest(), pin.file);
            let Ok(source) = std::fs::read_to_string(&path) else {
                errors.push(format!(
                    "claim `{}` pins {} which does not exist",
                    claim.id, pin.file
                ));
                continue;
            };
            let extracted: Vec<String> = match &pin.const_name {
                Some(name) => extract_consts(&source, name),
                None => Vec::new(),
            };
            match pin.relation.as_str() {
                "equals-canonical" => {
                    let one = extracted.first().cloned().unwrap_or_default();
                    let canonical = claim.canonical.clone().unwrap_or_default();
                    if flatten(&one) != flatten(&canonical) {
                        errors.push(format!(
                            "claim `{}` contradicts its dedicated guard: {} {} differs from the registered canonical",
                            claim.id, pin.file, pin.const_name.as_deref().unwrap_or("")
                        ));
                    }
                }
                "contained-in-canonical" => {
                    let one =
                        flatten(&extracted.first().cloned().unwrap_or_default()).to_lowercase();
                    let canonical =
                        flatten(claim.canonical.as_deref().unwrap_or("")).to_lowercase();
                    if one.is_empty() || !canonical.contains(&one) {
                        errors.push(format!(
                            "claim `{}` contradicts its dedicated guard: {} {} is no longer a phrase of the registered canonical",
                            claim.id, pin.file, pin.const_name.as_deref().unwrap_or("")
                        ));
                    }
                }
                "equals-fragments" => {
                    let mut got = extracted.clone();
                    let mut want = claim.canonical_fragments.clone();
                    got.sort();
                    want.sort();
                    if got != want {
                        errors.push(format!(
                            "claim `{}` contradicts its dedicated guard: {} {} differs from the registered canonical fragments",
                            claim.id, pin.file, pin.const_name.as_deref().unwrap_or("")
                        ));
                    }
                }
                "subset-of-signatures" => {
                    for value in &extracted {
                        if !claim.signatures.iter().any(|s| s == value) {
                            errors.push(format!(
                                "claim `{}` contradicts its dedicated guard: {} {value:?} is not among the registered signatures",
                                claim.id, pin.file
                            ));
                        }
                    }
                }
                "named-in-file" => {
                    // The dedicated guard asserts on one of the claim's exact
                    // signature bytes rather than a named constant; require at
                    // least one signature verbatim in the guard source.
                    let haystack = flatten(&source).to_lowercase();
                    let named = claim.signatures.iter().any(|signature| {
                        !signature.is_empty()
                            && haystack.contains(&flatten(signature).to_lowercase())
                    });
                    if !named {
                        errors.push(format!(
                            "claim `{}` contradicts its dedicated guard: {} no longer states the registered canonical bytes",
                            claim.id, pin.file
                        ));
                    }
                }
                unknown => errors.push(format!(
                    "claim `{}` uses unknown pin relation {unknown:?}",
                    claim.id
                )),
            }
        }
    }
    errors
}

/// Extract every string literal assigned to `const NAME` (handles multi-line
/// `\` continuations, semicolons inside literals, concat-style adjacency and
/// arrays of literals). Deliberately dumb, like the citation parsers elsewhere
/// in the suite.
fn extract_consts(source: &str, name: &str) -> Vec<String> {
    let marker = format!("const {name}");
    let Some(at) = source.find(&marker) else {
        return Vec::new();
    };
    let body = &source[at..];
    let Some(eq) = body.find('=') else {
        return Vec::new();
    };
    let mut values: Vec<String> = Vec::new();
    let mut chars = body[eq + 1..].char_indices().peekable();
    // Text seen between literals since the last one closed: a comma there
    // marks array elements, whitespace-only gaps mark concatenated literals.
    let mut gap = String::new();
    while let Some((_, ch)) = chars.next() {
        match ch {
            // A top-level semicolon ends the declaration; semicolons inside
            // literals are consumed by the literal scanner below.
            ';' => break,
            '"' => {
                let mut literal = String::new();
                while let Some((_, c)) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            // `\` + newline + indent joins the continuation line.
                            Some((_, '\n')) => {
                                while matches!(chars.peek(), Some((_, ' ' | '\t'))) {
                                    chars.next();
                                }
                            }
                            Some((_, escaped)) => {
                                literal.push('\\');
                                literal.push(escaped);
                            }
                            None => break,
                        },
                        c => literal.push(c),
                    }
                }
                if values.is_empty() || gap.contains(',') {
                    values.push(literal);
                } else {
                    values
                        .last_mut()
                        .expect("a literal precedes")
                        .push_str(&literal);
                }
                gap.clear();
            }
            c => gap.push(c),
        }
    }
    values
}

#[test]
fn registry_matches_what_the_surfaces_render_and_the_guards_pin() {
    let claims = registry();
    let errors = check_weld(&claims);
    assert!(
        errors.is_empty(),
        "the registry must equal what the owning surfaces render or emit:\n{}",
        errors.join("\n")
    );
}

// ---------------------------------------------------------------------------
// scenario: guard green on the tree
// ---------------------------------------------------------------------------

#[test]
fn guard_green_on_the_tree() {
    let claims = registry();
    let mut corpus = rendered_surfaces();
    let docs = payload_docs();
    assert!(
        corpus.len() >= 37 && docs.len() >= 80,
        "the scan must cover every rendered surface and every payload doc, saw {} surfaces and {} docs",
        corpus.len(),
        docs.len()
    );
    corpus.extend(docs);
    let violations = scan(&corpus, &claims);
    assert!(violations.is_empty(), "{}", render(&violations));
}

// ---------------------------------------------------------------------------
// scenario: a stale echo fails loudly (planted into scratch surfaces only)
// ---------------------------------------------------------------------------

#[test]
fn planted_echoes_fail_loudly_with_line_numbers() {
    let claims = registry();
    let mut corpus = rendered_surfaces();
    corpus.extend(payload_docs());

    // Scratch surfaces (dev-only, never shipped): four doc paraphrases, one per
    // family, planted at known lines, plus a drifted error quote and a
    // paraphrase appended to a synthetic help-topic surface.
    let planted_doc = "# scratch\n\na stable line\nthe driver derives the tier from the tree's own packages, unlike a workspace\nfollow the chain scan → report → diagram → verify before the strict gate\nnote: facade dependency rule inert for go: facade roles are not derivable there\nwhen no tier exists: depgraph needs the module tier, which this model has none of, so the model is healthy\n";
    corpus.push((
        "scratch/planted-guide.md".to_string(),
        planted_doc.to_string(),
    ));
    let planted_line = |needle: &str| -> usize {
        planted_doc
            .lines()
            .position(|line| line.contains(needle))
            .map(|at| at + 1)
            .expect("planted line")
    };

    let mut surfaces = rendered_surfaces();
    let help_constraints = surfaces
        .iter_mut()
        .find(|(name, _)| name == "help constraints")
        .expect("help constraints renders");
    help_constraints
        .1
        .push_str("\nThe quick audit runs scan -> report -> diagram -> verify, then strict.\n");
    corpus = surfaces;
    corpus.extend(payload_docs());
    corpus.push((
        "scratch/planted-guide.md".to_string(),
        planted_doc.to_string(),
    ));

    let violations = scan(&corpus, &claims);
    let rendered = render(&violations);
    for (claim, line) in [
        ("go-module-tier", planted_line("tree's own packages")),
        ("audit-recipe", planted_line("scan → report")),
        (
            "facade-inert-note",
            planted_line("facade dependency rule inert"),
        ),
        (
            "depgraph-refusal",
            planted_line("depgraph needs the module tier"),
        ),
    ] {
        let hits: Vec<&Violation> = violations
            .iter()
            .filter(|v| v.claim_id == claim && v.surface == "scratch/planted-guide.md")
            .collect();
        let lines: Vec<usize> = {
            let mut lines: Vec<usize> = hits.iter().map(|hit| hit.line).collect();
            lines.sort_unstable();
            lines.dedup();
            lines
        };
        assert_eq!(
            lines,
            vec![line],
            "the planted {claim} echo must fail on its line: {rendered}"
        );
        assert!(
            rendered.contains(&format!("claim `{claim}`")) && rendered.contains("owner:"),
            "the failure names the claim id and its owner surface: {rendered}"
        );
    }
    let recipe_in_help = violations
        .iter()
        .find(|v| v.claim_id == "audit-recipe" && v.surface == "help constraints")
        .expect("the planted help-text paraphrase must be detected");
    assert!(
        recipe_in_help.line > planted_doc.lines().count(),
        "the help-text violation carries a line of the rendered topic: {recipe_in_help:?}"
    );
    assert!(
        rendered.contains("restore the canonical bytes")
            || rendered.contains("citation of the owner"),
        "the message states the two allowed fixes: {rendered}"
    );
    assert!(
        rendered.contains("scan → report") || rendered.contains("scan -> report"),
        "the message quotes the offending chain bytes: {rendered}"
    );
    // The scratch corpus minus the plants is the clean tree again: the same
    // scan returns to green, proving the plants — not the machinery — failed.
    let mut clean = rendered_surfaces();
    clean.extend(payload_docs());
    assert!(
        scan(&clean, &claims).is_empty(),
        "clean tree must stay green"
    );
}

// ---------------------------------------------------------------------------
// scenario: registry drift (canonical edited in the registry only) fails
// ---------------------------------------------------------------------------

#[test]
fn registry_drift_fails_the_weld() {
    let mut claims = registry();
    let drifted = claims
        .iter_mut()
        .find(|c| c.id == "go-module-tier")
        .expect("go-module-tier is registered");
    drifted.canonical = Some(format!(
        "{} and additionally never at all",
        drifted.canonical.as_deref().unwrap_or("")
    ));
    let errors = check_weld(&claims);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("go-module-tier") && e.contains("drifted")),
        "a registry canonical that the owner surface does not render must fail naming the claim: {errors:?}"
    );

    let mut claims = registry();
    let drifted = claims
        .iter_mut()
        .find(|c| c.id == "roles-in-views")
        .expect("roles-in-views is registered");
    drifted.canonical_fragments = vec!["  inspect tree — marks: entrypoints only.".to_string()];
    let errors = check_weld(&claims);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("roles-in-views") && e.contains("dedicated guard")),
        "a registry fragment contradicting the matrix registry must fail: {errors:?}"
    );
    let mut claims = registry();
    let drifted = claims
        .iter_mut()
        .find(|c| c.id == "artefact-dirt-map")
        .expect("artefact-dirt-map is registered");
    drifted.canonical = Some(
        drifted
            .canonical
            .take()
            .expect("dirt-map canonical")
            .replace("keys on structure", "keys on declarations"),
    );
    let errors = check_weld(&claims);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("artefact-dirt-map") && e.contains("drifted")),
        "a registry canonical the owner surface no longer renders must fail naming the claim: {errors:?}"
    );
    assert!(
        check_weld(&registry()).is_empty(),
        "the shipped registry passes the weld"
    );
}

// ---------------------------------------------------------------------------
// scenario: the four known families cannot return
// ---------------------------------------------------------------------------

#[test]
fn the_four_known_families_cannot_return() {
    let claims = registry();
    let plants: Vec<(&str, &str)> = vec![
        (
            "audit-recipe",
            "the agents guide orders the audit: scan → report → diagram → verify\n",
        ),
        (
            "go-module-tier",
            "on a driver without a module tier, e.g. single-module go, the tier is never derived from the tree's own packages\n",
        ),
        (
            "roles-in-views",
            "a rust bin's `<unit>::main` entry marks in `inspect tree` alone\n",
        ),
        (
            "depgraph-refusal",
            "when no module tier exists depgraph needs the module tier, which this model has none of, because the tree is flat\n",
        ),
    ];
    let mut base = rendered_surfaces();
    base.extend(payload_docs());
    for (claim_id, plant) in &plants {
        let mut corpus = base.clone();
        corpus.push(("scratch/planted-family.md".to_string(), plant.to_string()));
        let violations = scan(&corpus, &claims);
        assert!(
            violations
                .iter()
                .any(|v| v.claim_id == *claim_id && v.surface == "scratch/planted-family.md"),
            "planting the {claim_id} family onto the clean tree must fail the generic scan naming the planted location and the claim id: {}",
            render(&violations)
        );
    }
    // Each family also stays dead through its pinned dedicated assertion: the
    // registry names the guard file, and that guard's source carries the
    // family bytes (its constants are byte-compared against the registry by
    // the weld test above).
    for claim in claims
        .iter()
        .filter(|c| plants.iter().any(|(id, _)| id == &c.id))
    {
        let guard = claim
            .dedicated_guard
            .clone()
            .unwrap_or_else(|| panic!("claim {} must name its dedicated guard", claim.id));
        let source = std::fs::read_to_string(format!("{}/{}", manifest(), guard))
            .unwrap_or_else(|_| panic!("dedicated guard {guard} exists"));
        let haystack = flatten(&source).to_lowercase();
        let carries = claim
            .canonical
            .as_ref()
            .is_some_and(|canonical| haystack.contains(&flatten(canonical).to_lowercase()))
            || claim
                .signatures
                .iter()
                .any(|signature| haystack.contains(&flatten(signature).to_lowercase()));
        assert!(
            carries,
            "the dedicated guard {guard} for claim `{}` must state the family bytes (canonical or signature)",
            claim.id
        );
    }
}

// ---------------------------------------------------------------------------
// scenario (blind4 follow-up US 06): wave-1..5 sentences are all registered —
// a planted stale echo of ANY canonical family fails the guard, so no wave's
// prose can drift silently again; the blind-5 follow-up US 05 registrations
// (check-render-guidance, first-spec-bootstrap, destination-canonical,
// artefact-dirt-map) grow one family each. The two template-form families
// (`wrote-echo`, `destination-statement`) cannot restate themselves without
// dropping the template bytes: an exact echo of them is legal anywhere BY
// DESIGN, so their drift trip is the dedicated guard pinning the emitted
// stdout — asserted structurally below, byte-tested inside the guards.
// ---------------------------------------------------------------------------

#[test]
fn wave_families_cannot_drift_silently() {
    let claims = registry();
    let plants: Vec<(&str, &str)> = vec![
        (
            "audit-recipe",
            "the agents guide orders the audit: scan → report → diagram → verify\n",
        ),
        (
            "go-module-tier",
            "the module tier derives from the tree's own packages on every driver\n",
        ),
        (
            "depgraph-refusal",
            "when no module tier exists: depgraph needs the module tier, which this model has none of, because the tree is flat\n",
        ),
        (
            "inspect-refusal",
            "on a flat tree, inspect tree needs the module tier, which this model has none of, so nothing renders\n",
        ),
        (
            "facade-inert-note",
            "note: facade dependency rule inert for go: facade roles are not derivable there\n",
        ),
        (
            "roles-in-views",
            "a rust bin's `<unit>::main` entry marks in `inspect tree` alone\n",
        ),
        (
            "api-usage-role-less-note",
            "the view prints note: this view shows no roles by decision, though the matrix still marks roles\n",
        ),
        (
            "depgraph-fold-note",
            "the modules view ends with a note that 3 role facts folded onto one node with different roles, so the node states nothing\n",
        ),
        (
            "rust-unit-naming",
            "a crate with lib.rs and main.rs yields `<pkg>` plus `<pkg>-bin` without any section\n",
        ),
        (
            "go-naming-legality",
            "a short package name does not resolve when `matches.units` targets a go unit\n",
        ),
        (
            "report-diagram-granularity",
            "the diagram embedded in a markdown report stays file-granular; the module-granular graph is a separate command\n",
        ),
        (
            "csharp-duality",
            "csharp projects carry two names for one project: a dotted build-unit name and a double-colon namespace name\n",
        ),
        (
            "fold-pointer",
            "unit ownership lives in the scan model's root module declarations, not in the folded graph labels\n",
        ),
        (
            "test-gated-edges",
            "test-gated/soft edges can appear in the graph because depgraph projections ignore the exclusion\n",
        ),
        (
            "mod-label",
            "call the root-module bucket what the graph labels mod for a unit\n",
        ),
        (
            "init-update-summary",
            "a newcomer runs update, which works by snapshotting the real tree, then tightens\n",
        ),
        (
            "exploratory-render",
            "an exploratory render keeps the committed artefact alone and prints to stdout\n",
        ),
        (
            "pretty-switch",
            "scan --pretty makes the model json indented while the default stays compact\n",
        ),
        (
            "check-ok-line",
            "a green check prints ok: build/report.md — up to date and nothing else\n",
        ),
        (
            "check-render-guidance",
            "a failing check compares the canonical rendering unless the artefact carries a --format header of its own\n",
        ),
        (
            "first-spec-bootstrap",
            "steps 1–2 create no spec: on a first capture of a fresh tree, run update once and keep the seed\n",
        ),
        (
            "destination-canonical",
            "the configured destination is provisioned for the default text rendering whatever the check compares\n",
        ),
        (
            "artefact-dirt-map",
            "freshness keys on structure: a comment dirties no artefact while a rename dirties none either\n",
        ),
    ];
    let mut base = rendered_surfaces();
    base.extend(payload_docs());
    for (claim_id, plant) in &plants {
        let mut corpus = base.clone();
        corpus.push((
            "scratch/planted-wave-family.md".to_string(),
            plant.to_string(),
        ));
        let violations = scan(&corpus, &claims);
        let rendered = render(&violations);
        assert!(
            violations
                .iter()
                .any(|v| v.claim_id == *claim_id && v.surface == "scratch/planted-wave-family.md"),
            "planting the {claim_id} family must fail the generic scan naming the planted location and the claim id: {rendered}"
        );
        assert!(
            rendered.contains(&format!("claim `{claim_id}`")) && rendered.contains("owner:"),
            "the failure names the claim id and its owner surface: {rendered}"
        );
        assert!(
            rendered.contains("citation of the owner") || rendered.contains("cite the owner"),
            "the failure states the allowed relations so the fix is derivable: {rendered}"
        );
    }
    for id in ["wrote-echo", "destination-statement"] {
        let claim = claims
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("template family `{id}` is registered"));
        let guard = claim
            .dedicated_guard
            .clone()
            .unwrap_or_else(|| panic!("claim {id} names its dedicated guard"));
        let source = std::fs::read_to_string(format!("{}/{}", manifest(), guard))
            .unwrap_or_else(|_| panic!("dedicated guard {guard} exists"));
        let haystack = flatten(&source).to_lowercase();
        let carries = claim
            .signatures
            .iter()
            .any(|signature| haystack.contains(&flatten(signature).to_lowercase()));
        assert!(
            carries,
            "template family `{id}` stays dead through its dedicated guard {guard} pinning the emitted stdout bytes",
        );
    }
}

// ---------------------------------------------------------------------------
// scenario (blind5 follow-up US 05, D06): the COMMANDS capability summary and
// the capability usage error share one grammar — the summary's sub-command
// forms and the refusal's forms are the same bytes, so the discovery surface
// and the refusal cannot drift apart.
// ---------------------------------------------------------------------------

#[test]
fn capability_summary_agrees_with_the_usage_error_grammar() {
    let fixture = Fixture::new();
    let help = surface(&fixture, &["--help"]);
    let line = help
        .lines()
        .find(|line| line.trim_start().starts_with("capability"))
        .unwrap_or_else(|| panic!("the COMMANDS index lists capability:\n{help}"));
    let refused = fixture.run(&["capability"]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "bare capability keeps its exit-1 usage error (D08)"
    );
    let usage = stderr(&refused);
    assert!(
        usage.starts_with("error: usage: archspec capability "),
        "the refusal stays a capability usage error: {usage}"
    );
    let grammar_after = |text: &str| -> String {
        let at = text
            .find("matrix")
            .unwrap_or_else(|| panic!("the sub-command grammar names matrix: {text}"));
        text[at..].split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let summary_grammar = grammar_after(line);
    let usage_grammar = grammar_after(&usage);
    assert_eq!(
        summary_grammar, usage_grammar,
        "the COMMANDS summary line and the capability usage error must state the same sub-command grammar"
    );
    assert!(
        summary_grammar.contains("matrix")
            && summary_grammar.contains("granular <language> <fact>"),
        "the shared grammar names both forms: {summary_grammar}"
    );
}
