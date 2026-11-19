use crate::archspec::capability;
use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language::{self, Language};
use crate::archspec::profile;
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::verify::compare;
use std::path::Path;

/// Sentinel returned as the Err payload when the full diff report (the product)
/// was already printed to stdout. Dispatch must exit non-zero without echoing
/// it to stderr — a rule violation is not an operational error.
pub const RULE_VIOLATION_REPORTED: &str = "\u{1}archspec-rule-violation-reported";

pub const HELP: &str = "usage: archspec verify [path] [--strict] [--json] [--coverage] [--only <class>] [--write-exemptions] [--emit-repro]\nextract the model and compare against architecture.spec.toml; exit 1 on violations\n\n  path          project directory to extract and compare (default: current directory)\n  --strict      promote every warning to an error (CI gate); a green run means\n                zero findings of any kind — except the async census rows, which\n                never promote even under --strict\n  --json        emit the verdict as machine-readable json: the metrics and findings\n                body of `archspec report --format json` — the same findings, the same\n                verdict, the same exit codes; a renderer, not a second adjudication\n  --coverage    print the coverage census after the verdict: one line per constraint\n                row stating the facts its check adjudicates in its declared territory\n                versus the model totals; with the [[exempt]] ratchet active one\n                further line states `coverage: cycles: <found>, exempt: <E>,\n                violating: <V>`\n  --only <class>  hide one diagnostic class from the report-only notices; findings,\n                the verdict and the environment census are unchanged; the run echoes\n                `only: excluding class \"<class>\" — verdict unchanged`\n  --write-exemptions  seed the [[exempt]] ledger: merge one entry per currently\n                violating cycle (identity + tier) into the ledger tables, never\n                edit or remove an existing entry, never auto-grow a covered one;\n                an entry whose keys the writer cannot verify is named on stderr\n                and left untouched; the run echoes `wrote <path>` and goes green\n                for what it just named. Refused with --json (64): a machine\n                consumer must never trigger a write\n  --emit-repro  print an import-order repro section after the verdict: one\n                greppable `repro:` line per runtime file cycle states the ring\n                path, then runnable node one-liners for both import orders with\n                the file each order is expected to fail in; type-assisted rings\n                print nothing; omitted with --json, stdout stays the machine body\n\nEvery verdict declares the environment it was adjudicated against: the line\n`adjudicated against BUILT tree`, or `adjudicated against UNBUILT tree — N\nspecifiers resolved to src instead of dist` when the scan resolved declared\nbuild targets to source. `--json` states the same fact as the `environment`\nfield; a summary-by-class census follows the header whenever the scan\nemitted report-only lines.\n\nsee 'archspec help workflow' for the audit recipe and 'archspec help diagnostics'\nfor what each finding category means\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "strict",
                takes_value: false,
            },
            FlagSpec {
                name: "json",
                takes_value: false,
            },
            FlagSpec {
                name: "coverage",
                takes_value: false,
            },
            FlagSpec {
                name: "only",
                takes_value: true,
            },
            FlagSpec {
                name: "write-exemptions",
                takes_value: false,
            },
            FlagSpec {
                name: "emit-repro",
                takes_value: false,
            },
        ],
    )?;
    cli::exactly_one_positional(&parsed)?;
    // `--json` + `--write-exemptions` is a usage error (GitHub issue #27,
    // R8): a machine consumer must never trigger a write.
    if parsed.switches.contains("json") && parsed.switches.contains("write-exemptions") {
        return Err(cli::usage_error(
            "--json and --write-exemptions cannot be combined — a machine consumer \
             must never trigger a write",
        ));
    }

    let root = parsed
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or(".");
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {root}"));
    }

    let mut loaded = profile::phase("spec-load", || -> Result<spec::Spec, String> {
        let loaded = spec::load_verify(root_path)?;
        // Every run names every active exclusion on the report surface (stderr)
        // before the verdict prints — an exclusion can never shrink the analyzed
        // set in silence (D13/R3-7).
        eprint!("{}", spec::excluded_sources_report(&loaded));
        // A dead entry — one that matched nothing under the mechanical set —
        // is stated on the same surface right after the active-entry lines
        // (issue-8 phase 2, decision-check MAJOR-2): once per entry per run,
        // never a silent pass.
        eprint!("{}", spec::exclusion_liveness_report(root_path, &loaded));
        Ok(loaded)
    })?;
    let language = language_from_str(&loaded.project.language)?;

    // Root-marker selection (detection record GC): the pin SELECTS the
    // DRIVER — its root marker wins outright, a root carrying only foreign
    // markers answers the `language mismatch:` branch (byte prefix kept),
    // a marker-less root keeps the unchanged zero-hits guard. The
    // nested-marker report line prints BEFORE the agreement precondition
    // so a red run carries it too.
    let language = language::select(root_path, Some(language))?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    eprint!("{}", language::marker_report(root_path, language));

    // The agreement check is a precondition (D14): the declared language is
    // the truth of the tree, and any un-excluded foreign source file fails
    // the run loud BEFORE any verdict, note or artifact is computed.
    if let Some(finding) = language::agreement_finding(
        root_path,
        language,
        &spec::exclusion_paths(&loaded),
    ) {
        return Err(finding);
    }
    if !scan::driver_available(language) {
        return Err(format!(
            "no {} toolchain found (driver missing); run 'archspec doctor' to diagnose",
            language_display(language)
        ));
    }

    let mut model = profile::phase("scan", || scan::extract_checked(language, root_path))?;
    // The scan report surface covers both verbs (D13): wildcard facts print
    // here exactly as they do on `scan`, report-only (R2-6), and so do the
    // `external not installed` lines (ts-resolution record R7b).
    eprint!("{}", scan::wildcard_report(&model));
    // The environment declaration (GitHub issue #24): the census counts exactly
    // the lines this run emits on the report surface — a count over facts
    // already emitted, never a second parse of the tree. `--only <class>` hides
    // one diagnostic class from the notices: findings, the verdict and the
    // census are unchanged, and the exclusion is always echoed, even when the
    // class names nothing.
    let environment = crate::archspec::report::environment(&model);
    let (notices, echo) = crate::archspec::report::filtered_report_only_lines(
        &model,
        parsed.values.get("only").map(String::as_str),
    );
    if let Some(echo) = &echo {
        eprintln!("{echo}");
    }
    eprint!("{notices}");
    // The warn-mode exclusion findings (issue-8 phase 2, decision-check
    // BLOCKER-2) fold into the diff's warnings channel below — never the
    // report-only channel, which can never go red under `--strict`.
    let mut scan_warnings = std::mem::take(&mut model.warnings);

    let mapping = profile::phase("mapping", || compare::map_units(&model.units, &loaded.modules));
    let mut diff = profile::phase("compare", || {
        compare::compare(
            &mapping,
            &model,
            &loaded.modules,
            &loaded.stereotype,
            &loaded.constraint,
            &loaded.exempt,
            &loaded.async_depends,
        )
    });
    // `--write-exemptions` (GitHub issue #27, R8; merge semantics per GitHub
    // issue #40): seed the ledger with one entry per currently violating
    // cycle, merging into the existing `[[exempt]]` tables — existing entries
    // and their comments are byte-preserved, covered violations re-seed
    // nothing. The write happens BEFORE the verdict renders: after a real
    // write the verdict is recomputed over the ledger as it now stands, so
    // the seeding run honestly reports the exemptions it just named (the
    // file, not the memory, is the ledger).
    if parsed.switches.contains("write-exemptions") {
        let seeds = seed_entries(&loaded.exempt, &diff.cycle_debts);
        if !seeds.is_empty() {
            let spec_path = root_path.join(spec::SPEC_FILE);
            if seed_exempt_entries(&spec_path, &seeds)? > 0 {
                println!("wrote {}", spec_path.display());
                loaded = spec::load_verify(root_path)?;
                diff = profile::phase("compare", || {
                    compare::compare(
                        &mapping,
                        &model,
                        &loaded.modules,
                        &loaded.stereotype,
                        &loaded.constraint,
                        &loaded.exempt,
                        &loaded.async_depends,
                    )
                });
            }
        }
    }
    // The fold (issue-8 phase 2): scan-side warn-mode findings join the
    // spec-side warnings in the diff's warnings channel, listed after all
    // error items and gated by `--strict` exactly like every other warning.
    diff.warnings.append(&mut scan_warnings);
    diff.warnings.sort();
    diff.warnings.dedup();
    // `--strict` promotes every warning-level divergence to an error so any
    // finding fails the run (design §9, cli.md: the CI gate).
    let strict = parsed.switches.contains("strict");
    let json = parsed.switches.contains("json");
    let coverage = parsed.switches.contains("coverage");
    let notes = inert_rule_notes(language, &diff, &loaded.async_depends, &model);

    // One adjudication, two renderers (GitHub issue #25): the verdict below is
    // computed once from this run's diff; `--json` only replaces the stdout
    // body with the report verb's machine form — the very
    // `crate::archspec::report::render_json` the report command calls, over
    // this run's own model, mapping and diff, so the findings, the verdict and
    // the exit code are the text run's, never a second adjudication. The
    // informational notes are text-surface output: a json run omits them so
    // stdout stays the pure machine body (the report D02 byte-clean law), and
    // the capability facts they state stay readable from
    // `archspec capability matrix`.
    profile::phase("report", || {
        if json {
            let metrics =
                crate::archspec::report::compute_metrics(&model, &mapping, &loaded.modules, &diff);
            // The machine verdict body carries the adjudicated PAIR census
            // (workplan archspec_async_depends, D10): the same numbers the
            // report's census section renders, computed over the same
            // declared table this run adjudicated; empty table, absent field.
            let async_census =
                crate::archspec::report::AsyncCensus::from_declared(&loaded.async_depends);
            print!(
                "{}",
                crate::archspec::report::render_json(
                    &loaded.project.language,
                    &metrics,
                    &diff,
                    &model.roles,
                    &environment,
                    &async_census,
                )
            );
        } else if diff.is_empty() {
            print!(
                "{}",
                compare::pass_confirmation(&loaded.modules, loaded.constraint.len())
            );
            // The exempt ratchet never passes in silence (GitHub issue #27, R5):
            // with exemptions active the census rides the pass confirmation too,
            // stating the suppressed rings as exempt, never a lying zero.
            if diff.cycles_exempt > 0
                && let Some(census) = &diff.cycles_census
            {
                println!("  {census}");
            }
            print_notes(&notes);
        } else {
            // Warning-only runs still print the full report on stdout; without
            // --strict the warnings are tolerated, so such a run passes (exit 0).
            print!("{}", compare::render_report(&diff, strict, !diff.fails(strict)));
            print_notes(&notes);
        }
        // The environment declaration (GitHub issue #24) rides the verdict surface
        // on every non-json path: the header states the build state the verdict was
        // adjudicated against, the census table follows wherever the scan emitted
        // report-only lines. A json run states the same fact in its `environment`
        // field and keeps stdout byte-clean.
        if !json {
            print!("{}", crate::archspec::report::environment_block(&environment));
            // The import-order repro section (GitHub issue #28): one greppable
            // `repro:` block per violating runtime file-tier cycle — the ring
            // path plus BOTH import orders as runnable node one-liners with
            // their TDZ expectations. Pure output: it renders the findings this
            // run already made, writes no file and moves no verdict byte; a
            // json run omits it so stdout stays the pure machine body.
            if parsed.switches.contains("emit-repro") {
                print!("{}", compare::cycle_repro_report(language.as_str(), &diff.cycle_rings));
            }
        }
        // The coverage census (GitHub issue #23) rides the verdict surface on
        // stdout: the verdict body first, the census after it, on every verdict
        // path.
        if coverage {
            print!(
                "{}",
                compare::coverage_report(&model, &loaded.modules, &loaded.constraint)
            );
            // The exempt ratchet split on the coverage surface (GitHub issue
            // #27, R5): printed only with exemptions active — without them the
            // coverage bytes are the shipped ones.
            if diff.cycles_exempt > 0 {
                println!(
                    "coverage: cycles: {}, exempt: {}, violating: {}",
                    diff.cycles_exempt + diff.cycles.len()
                        + diff.warnings.iter().filter(|w| w.starts_with("cycle: ")).count(),
                    diff.cycles_exempt,
                    diff.cycles.len()
                        + diff.warnings.iter().filter(|w| w.starts_with("cycle: ")).count(),
                );
            }
        }
    });
    if diff.fails(strict) {
        return Err(RULE_VIOLATION_REPORTED.to_string());
    }
    Ok(())
}

/// The `--write-exemptions` seed set (GitHub issue #27, R8): violating
/// `(tier, identity)` debts not already covered by the ledger. Coverage
/// ignores expiry on purpose — an expired entry is a human-owned fact the
/// writer must never paper over with a duplicate identity (the duplicate
/// would be refused by `validate_exempt`; the warning the expired entry
/// prints is the honest channel).
fn seed_entries(ledger: &[spec::Exempt], debts: &[(String, String)]) -> Vec<(String, String)> {
    debts
        .iter()
        .filter(|(tier, identity)| {
            !ledger.iter().any(|entry| {
                entry.cycle.as_deref().map(str::trim) == Some(identity.as_str())
                    && entry.matches_tier(tier)
            })
        })
        .cloned()
        .collect()
}

/// The `--write-exemptions` seed writer (GitHub issue #27, R8; merge
/// semantics per GitHub issue #40): seeds MERGE into the existing
/// `[[exempt]]` tables — the file is never re-serialized, and every existing
/// table keeps its bytes, its documented optional keys (`count`, `until`,
/// `reason`) and the comments around it. New entries carry the machine-known
/// facts only: `cycle` and `tier`; the human-owned keys stay absent until a
/// human adds them. Returns how many entries the write added (0 when every
/// seed merged into an existing entry or the merged bytes were refused).
fn seed_exempt_entries(spec_path: &Path, seeds: &[(String, String)]) -> Result<usize, String> {
    let text = std::fs::read_to_string(spec_path)
        .map_err(|e| format!("cannot read {}: {e}", spec_path.display()))?;
    let tables = scan_exempt_tables(&text);
    for table in tables
        .iter()
        .filter(|table| matches!(table.body, LedgerTableBody::Unverifiable))
    {
        eprintln!(
            "warning: exempt entry #{} has ledger keys the seeding writer cannot read \
             — its bytes are left untouched",
            table.index
        );
    }
    // Merge by the entry identity (cycle, tier): a seed the raw ledger text
    // already carries is skipped — an annotated table is never rewritten
    // beside its own identity. Model coverage filters covered seeds already;
    // this is the structural guard (GitHub issue #40).
    let seeds: Vec<&(String, String)> = seeds
        .iter()
        .filter(|(tier, identity)| {
            !tables.iter().any(|table| {
                matches!(&table.body, LedgerTableBody::Identity { cycle, tier: entry_tier }
                    if cycle == identity && (entry_tier == tier || entry_tier == "both"))
            })
        })
        .collect();
    if seeds.is_empty() {
        return Ok(0);
    }
    // Assemble the merged text: new entries join the ledger region — just
    // after the last `[[exempt]]` table when the file has one, at end of file
    // otherwise (R8's append shape). Bytes outside the inserted text never
    // move, so comments outside a table stay where they are.
    let insert_at = tables.last().map_or(text.len(), |table| table.end);
    let mut out = String::new();
    if insert_at < text.len() {
        let (before, after) = text.split_at(insert_at);
        out.push_str(before.trim_end_matches([' ', '\t', '\n']));
        for (tier, identity) in &seeds {
            out.push_str(&format!(
                "\n\n[[exempt]]\ncycle = \"{identity}\"\ntier = \"{tier}\"\n"
            ));
        }
        if !after.is_empty() {
            out.push('\n');
            out.push_str(after.trim_start_matches('\n'));
        }
    } else {
        out.push_str(&text);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        for (tier, identity) in &seeds {
            out.push_str(&format!(
                "\n[[exempt]]\ncycle = \"{identity}\"\ntier = \"{tier}\"\n"
            ));
        }
    }
    // Fail closed: the merged bytes must parse as TOML before they replace
    // the file — a scanner that misread the ledger warns and never corrupts
    // the ledger (GitHub issue #40: never lose data silently).
    if toml::from_str::<toml::Value>(&out).is_err() {
        eprintln!(
            "warning: seeding refused — the merged ledger does not parse; {} is unchanged",
            spec_path.display()
        );
        return Ok(0);
    }
    std::fs::write(spec_path, &out)
        .map_err(|e| format!("cannot write {}: {e}", spec_path.display()))?;
    Ok(seeds.len())
}

/// The body of one `[[exempt]]` table as read from the raw ledger text.
enum LedgerTableBody {
    /// The identity read from the table's single-line string keys — escaped
    /// and multi-line values are never guessed.
    Identity { cycle: String, tier: String },
    /// A key the text scanner cannot read exactly. The seeding writer leaves
    /// such a table untouched and names it on stderr (GitHub issue #40).
    Unverifiable,
}

/// One `[[exempt]]` table region of the raw ledger text.
struct LedgerTable {
    /// The table's ledger position, 1-based — the number `validate_exempt`
    /// words its messages with.
    index: usize,
    /// Byte offset just after the table region: the start of the next
    /// top-level table header, or the end of the file.
    end: usize,
    body: LedgerTableBody,
}

/// Locate every `[[exempt]]` table in the raw spec text: a line-oriented
/// scan of top-level headers, aware of multi-line strings. The scan is
/// best-effort by design — every risk it takes ends in the fail-closed
/// parse check of `seed_exempt_entries`, never in a corrupt write.
fn scan_exempt_tables(text: &str) -> Vec<LedgerTable> {
    let mut headers: Vec<(usize, bool)> = Vec::new();
    let mut open_string: Option<&str> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(delimiter) = open_string {
            if line.matches(delimiter).count() % 2 == 1 {
                open_string = None;
            }
        } else if !trimmed.starts_with('#') {
            if let Some(rest) = trimmed.strip_prefix("[[") {
                let name = rest.split("]]").next().unwrap_or_default().trim();
                headers.push((offset, name == "exempt"));
            } else if trimmed.starts_with('[') {
                headers.push((offset, false));
            }
            open_string = multiline_opener(line);
        }
        offset += line.len();
    }
    let mut tables = Vec::new();
    for (position, (start, is_exempt)) in headers.iter().enumerate() {
        if !*is_exempt {
            continue;
        }
        let end = headers
            .get(position + 1)
            .map_or(text.len(), |(next, _)| *next);
        tables.push(LedgerTable {
            index: tables.len() + 1,
            end,
            body: exempt_table_body(&text[*start..end]),
        });
    }
    tables
}

/// The first multi-line string delimiter (`\"\"\"` or `'''`) that stays open
/// on a line, if any.
fn multiline_opener(line: &str) -> Option<&'static str> {
    if line.matches("\"\"\"").count() % 2 == 1 {
        Some("\"\"\"")
    } else if line.matches("'''").count() % 2 == 1 {
        Some("'''")
    } else {
        None
    }
}

/// Read one exempt table's identity from its region text: single-line string
/// values for `cycle` and `tier`, no escapes; anything else is unverifiable.
fn exempt_table_body(region: &str) -> LedgerTableBody {
    // None: the key never appeared. Some(None): the key appeared but its
    // value cannot be read exactly. Some(Some(v)): the parsed text value.
    let mut cycle: Option<Option<String>> = None;
    let mut tier: Option<Option<String>> = None;
    let mut open_string: Option<&str> = None;
    for line in region.split_inclusive('\n').skip(1) {
        let trimmed = line.trim_start();
        if let Some(delimiter) = open_string {
            if line.matches(delimiter).count() % 2 == 1 {
                open_string = None;
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return LedgerTableBody::Unverifiable;
        };
        let slot = match key.trim() {
            "cycle" => &mut cycle,
            "tier" => &mut tier,
            _ => continue,
        };
        let value = strip_inline_comment(value.trim());
        if slot.is_some() {
            // A repeated key is a ledger the writer cannot account for —
            // the loader refuses the file before this point anyway.
            *slot = Some(None);
            continue;
        }
        if let Some(delimiter) = multiline_opener(value) {
            *slot = Some(None);
            open_string = Some(delimiter);
            continue;
        }
        *slot = Some(single_line_string(value));
    }
    match (cycle, tier) {
        (Some(Some(cycle)), Some(Some(tier))) => LedgerTableBody::Identity { cycle, tier },
        _ => LedgerTableBody::Unverifiable,
    }
}

/// Read a single-line TOML string value: basic (no escapes) or literal.
/// `None` marks a value whose text cannot be compared to a parsed identity
/// directly — an escape, a multi-line opener or a non-string value.
fn single_line_string(value: &str) -> Option<String> {
    if value.starts_with("\"\"\"") || value.starts_with("'''") {
        return None;
    }
    if let Some(rest) = value.strip_prefix('"') {
        let inner = rest.strip_suffix('"')?;
        if inner.contains('\\') {
            return None;
        }
        Some(inner.trim().to_string())
    } else if let Some(rest) = value.strip_prefix('\'') {
        let inner = rest.strip_suffix('\'')?;
        Some(inner.trim().to_string())
    } else {
        None
    }
}

/// Drop a trailing `#` comment that starts outside the quoted value.
fn strip_inline_comment(value: &str) -> &str {
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    for (position, byte) in value.bytes().enumerate() {
        match quote {
            None => match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'#' => return value[..position].trim_end(),
                _ => {}
            },
            Some(quote_char) => {
                if escaped {
                    escaped = false;
                } else if byte == quote_char {
                    quote = None;
                } else if quote_char == b'"' && byte == b'\\' {
                    escaped = true;
                }
            }
        }
    }
    value
}

/// Informational notes stating which capability-table-listed rules can never
/// fire for this language (or, for a conditional fact, fired this run from no
/// native scan fact). A note is suppressed whenever the rule's own finding is
/// present — a rule that just fired is not inert, the finding states what it
/// saw. Notes are output, not findings: they never enter the diff, never
/// change the verdict or exit status, and keep rust runs byte-identical (rust
/// emits every rule fact).
fn inert_rule_notes(
    language: Language,
    diff: &compare::Diff,
    async_depends: &[spec::AsyncDepend],
    model: &crate::archspec::model::Model,
) -> Vec<String> {
    let mut notes = Vec::new();
    for (rule, fact) in capability::RULES {
        if rule_reported(rule, diff) {
            continue;
        }
        if let Some(capability::NOT_EMITTED) = capability::emission(language, fact) {
            notes.push(format!(
                "note: {rule} rule inert for {}: no derivable {fact} idiom — alias \
                 umbrella, public-package and root delegation forms investigated, \
                 recorded in ADR-017",
                language.as_str()
            ));
        }
    }
    // The `[[async_depends]]` inert arms (workplan archspec_async_depends,
    // D3): stated only when the table is non-empty — an absent table keeps
    // the shipped bytes (D8). Arms (b)/(c) state the driver's capability
    // row; arm (a) states the model's census silence. A note is suppressed
    // whenever its own findings fired — a fired arm is not inert.
    if !async_depends.is_empty() {
        if capability::emission(language, capability::FACT_EVENT_DECLARATION_DERIVATION)
            == Some(capability::NOT_EMITTED)
            && diff.async_publish_not_derived.is_empty()
            && diff.async_subscribe_not_derived.is_empty()
        {
            notes.push(format!(
                "note: async_depends publish and subscribe arms inert for {}: \
                 event-declaration-derivation = not-emitted",
                language.as_str()
            ));
        }
        if !compare::census_present(model)
            && diff.async_event_unresolved.is_empty()
        {
            notes.push(
                "note: async_depends event-type arm inert: the model states no \
                 declared-type census facts"
                    .to_string(),
            );
        }
    }
    notes
}

/// True when the run's diff carries at least one finding of this rule, so an
/// accompanying "inert"-style note would contradict the report.
fn rule_reported(rule: &str, diff: &compare::Diff) -> bool {
    match rule {
        capability::RULE_FACADE_DEPENDENCY => !diff.facade_dependencies.is_empty(),
        capability::RULE_LAUNDERED_FORBIDDEN_EDGE => diff
            .warnings
            .iter()
            .any(|entry| entry.starts_with(&format!("{rule}: "))),
        _ => false,
    }
}

fn print_notes(notes: &[String]) {
    for note in notes {
        println!("{note}");
    }
}

pub(crate) fn language_from_str(value: &str) -> Result<Language, String> {
    // The shared parser (round-8 F11): the hand-copied third mapping is
    // deleted — the red bytes stay byte-identical.
    language::from_name(value)
        .ok_or_else(|| format!("invalid spec: unknown language \"{value}\""))
}

pub(crate) fn language_display(language: Language) -> String {
    let value = language.as_str();
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => value.to_string(),
    }
}
