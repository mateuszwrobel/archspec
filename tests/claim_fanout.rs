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

use common::{stderr, stdout, Fixture};
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
            relations: entry.get("relations").map_or(vec!["echo".into(), "cite".into()], |v| {
                vec_or_default(v)
            }),
            canonical: entry.get("canonical").and_then(|v| v.as_str()).map(str::to_string),
            canonical_fragments: string_list_opt(&entry.get("canonical_fragments").cloned().unwrap_or(toml::Value::Array(vec![]))),
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
    value.as_array().map(|_| string_list(value)).unwrap_or_default()
}

fn vec_or_default(value: &toml::Value) -> Vec<String> {
    string_list_opt(value)
}

// ---------------------------------------------------------------------------
// surfaces and docs
// ---------------------------------------------------------------------------

/// Every text surface an agent can read from the binary: the topic index, each
/// manual topic, each command through both `help <cmd>` and `<cmd> --help`,
/// the capability matrix, and the skill document.
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
        surfaces.push((format!("help {command}"), surface(&fixture, &["help", command])));
        surfaces.push((
            format!("{command} --help"),
            surface(&fixture, &[command, "--help"]),
        ));
    }
    surfaces.push(("skill".to_string(), surface(&fixture, &["skill"])));
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
        || matches!(ch, '/' | '.' | '-' | '_' | ':' | '<' | '>' | '$' | '@' | '~' | '\\')
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
                    let own_span = spans.iter().any(|(s, e, id)| *s <= start && end <= *e && *id == claim.id);
                    let other_span = spans.iter().any(|(s, e, id)| *s <= start && end <= *e && *id != claim.id);
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
                        canonical: claim.canonical.clone().unwrap_or_else(|| {
                            claim.canonical_fragments.join(" / ")
                        }),
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
    let required: [&str; 24] = [
        "audit-recipe",
        "go-module-tier",
        "roles-in-views",
        "facade-inert-note",
        "depgraph-refusal",
        "inspect-refusal",
        "api-usage-role-less-note",
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
        if let Some(canonical) = &claim.canonical {
            if !claim.template && !text.contains(&flatten(canonical).to_lowercase()) {
                errors.push(format!(
                    "claim `{}` drifted: `archspec {key}` does not render the registered canonical text",
                    claim.id
                ));
            }
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
                    let one = flatten(&extracted.first().cloned().unwrap_or_default()).to_lowercase();
                    let canonical = flatten(claim.canonical.as_deref().unwrap_or("")).to_lowercase();
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
                    values.last_mut().expect("a literal precedes").push_str(&literal);
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
    corpus.push(("scratch/planted-guide.md".to_string(), planted_doc.to_string()));
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
    help_constraints.1.push_str("\nThe quick audit runs scan -> report -> diagram -> verify, then strict.\n");
    corpus = surfaces;
    corpus.extend(payload_docs());
    corpus.push(("scratch/planted-guide.md".to_string(), planted_doc.to_string()));

    let violations = scan(&corpus, &claims);
    let rendered = render(&violations);
    for (claim, line) in [
        ("go-module-tier", planted_line("tree's own packages")),
        ("audit-recipe", planted_line("scan → report")),
        ("facade-inert-note", planted_line("facade dependency rule inert")),
        ("depgraph-refusal", planted_line("depgraph needs the module tier")),
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
        rendered.contains("restore the canonical bytes") || rendered.contains("citation of the owner"),
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
    assert!(scan(&clean, &claims).is_empty(), "clean tree must stay green");
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
        errors.iter().any(|e| e.contains("go-module-tier") && e.contains("drifted")),
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
    for claim in claims.iter().filter(|c| plants.iter().any(|(id, _)| id == &c.id)) {
        let guard = claim
            .dedicated_guard
            .clone()
            .unwrap_or_else(|| panic!("claim {} must name its dedicated guard", claim.id));
        let source = std::fs::read_to_string(format!("{}/{}", manifest(), guard))
            .unwrap_or_else(|_| panic!("dedicated guard {guard} exists"));
        let haystack = flatten(&source).to_lowercase();
        let carries = claim.canonical.as_ref().is_some_and(|canonical| {
            haystack.contains(&flatten(canonical).to_lowercase())
        }) || claim
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
        ("audit-recipe", "the agents guide orders the audit: scan → report → diagram → verify\n"),
        ("go-module-tier", "the module tier derives from the tree's own packages on every driver\n"),
        ("depgraph-refusal", "when no module tier exists: depgraph needs the module tier, which this model has none of, because the tree is flat\n"),
        ("inspect-refusal", "on a flat tree, inspect tree needs the module tier, which this model has none of, so nothing renders\n"),
        ("facade-inert-note", "note: facade dependency rule inert for go: facade roles are not derivable there\n"),
        ("roles-in-views", "a rust bin's `<unit>::main` entry marks in `inspect tree` alone\n"),
        ("api-usage-role-less-note", "the view prints note: this view shows no roles by decision, though the matrix still marks roles\n"),
        ("rust-unit-naming", "a crate with lib.rs and main.rs yields `<pkg>` plus `<pkg>-bin` without any section\n"),
        ("go-naming-legality", "a short package name does not resolve when `matches.units` targets a go unit\n"),
        ("report-diagram-granularity", "the diagram embedded in a markdown report stays file-granular; the module-granular graph is a separate command\n"),
        ("csharp-duality", "csharp projects carry two names for one project: a dotted build-unit name and a double-colon namespace name\n"),
        ("fold-pointer", "unit ownership lives in the scan model's root module declarations, not in the folded graph labels\n"),
        ("test-gated-edges", "test-gated/soft edges can appear in the graph because depgraph projections ignore the exclusion\n"),
        ("mod-label", "call the root-module bucket what the graph labels mod for a unit\n"),
        ("init-update-summary", "a newcomer runs update, which works by snapshotting the real tree, then tightens\n"),
        ("exploratory-render", "an exploratory render keeps the committed artefact alone and prints to stdout\n"),
        ("pretty-switch", "scan --pretty makes the model json indented while the default stays compact\n"),
        ("check-ok-line", "a green check prints ok: build/report.md — up to date and nothing else\n"),
        ("check-render-guidance", "a failing check compares the canonical rendering unless the artefact carries a --format header of its own\n"),
        ("first-spec-bootstrap", "steps 1–2 create no spec: on a first capture of a fresh tree, run update once and keep the seed\n"),
        ("destination-canonical", "the configured destination is provisioned for the default text rendering whatever the check compares\n"),
        ("artefact-dirt-map", "freshness keys on structure: a comment dirties no artefact while a rename dirties none either\n"),
    ];
    let mut base = rendered_surfaces();
    base.extend(payload_docs());
    for (claim_id, plant) in &plants {
        let mut corpus = base.clone();
        corpus.push(("scratch/planted-wave-family.md".to_string(), plant.to_string()));
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
