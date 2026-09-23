//! Release prep for 0.5.0 (blind5 follow-up US_06).
//!
//! The 0.5.0 tag carried an entire era under one version line whose trailing
//! comment chain was itself the changelog nobody read. This suite pins the
//! three payload data surfaces the release-prep commit lands: the CHANGELOG
//! opens its existing `[0.5.0]` section with a rollup of the era families
//! (the publish workflow's release-notes extractor reads the FIRST `## [0.5.0]`
//! heading to the next `## [` — so a rollup heading placed before the existing
//! one would silently drop every older bullet from the generated notes, and
//! the extractor's byte path is replayed here), the Cargo.toml version line
//! is one normal line with one release-marker comment (value stays 0.5.0,
//! the tag number), and `docs/archspec/agents.md` states the binary source —
//! build from source plus the release-tag asset names the payload's own
//! `release.yml` emits — in URL-free, payload-neutral wording (the docs tree
//! spells no URLs at all on this surface, so no owner-qualified repo URL can
//! collide with the publish workflow's leak guard, which stays the backstop).
//! Rule: D07 of `rust-arch-test-kit/worklog/workplan_archspec_blind5_followup/sources.md`.

fn manifest() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn load(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", manifest()))
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// The asset names exactly as `docs/archspec/agents.md` states them, paired
/// with the exact bytes the payload's `release.yml` assigns them — doc line
/// and release machinery cite the same bytes (a rename must move both in one
/// commit). `<version>` is the doc's placeholder for the `$ver` token.
const RELEASE_ASSETS: &[(&str, &str)] = &[
    (
        "archspec-<version>-x86_64-unknown-linux-musl.tar.gz",
        "archspec-${ver}-x86_64-unknown-linux-musl.tar.gz",
    ),
    (
        "archspec-<version>-aarch64-unknown-linux-gnu.tar.gz",
        "archspec-${ver}-aarch64-unknown-linux-gnu.tar.gz",
    ),
    (
        "archspec-<version>-x86_64-apple-darwin.tar.gz",
        "archspec-${ver}-x86_64-apple-darwin.tar.gz",
    ),
    (
        "archspec-<version>-aarch64-apple-darwin.tar.gz",
        "archspec-${ver}-aarch64-apple-darwin.tar.gz",
    ),
    (
        "archspec_<version>_amd64.deb",
        "archspec_${ver}_amd64.deb",
    ),
    (
        "archspec_<version>_arm64.deb",
        "archspec_${ver}_arm64.deb",
    ),
    (
        "archspec-<version>-1-x86_64.pkg.tar.zst",
        "archspec-${ver}-1-x86_64.pkg.tar.zst",
    ),
    (
        "archspec-<version>-1-aarch64.pkg.tar.zst",
        "archspec-${ver}-1-aarch64.pkg.tar.zst",
    ),
    (
        "archspec-<version>-x86_64-pc-windows-msvc.zip",
        "archspec-$ver-x86_64-pc-windows-msvc.zip",
    ),
];

/// The collapsed version line, pinned byte-exact: one line, the tag-number
/// value, one short release-marker comment.
const VERSION_LINE: &str = "version = \"0.5.0\"  # released — entry lives in CHANGELOG.md under [0.5.0]; do not bump past the tag until the release commit; the 0.5.0 retag carries the aarch64-musl release-build fix; this tree adds the Apache-2.0 license file and the generated credits listing at an unchanged version";

/// The rollup's opening line inside the `## [0.5.0]` section.
const ROLLUP_MARKER: &str = "Version 0.5.0 — released from this entry.";

/// Scenario "CHANGELOG carries the 0.5.0 rollup, old bullets intact": the
/// rollup lives INSIDE the existing section, the section's FIRST heading is
/// the one the extractor keys on, and the extractor's own byte path — first
/// `## [0.5.0]` heading to the next `## [` — yields the rollup AND every
/// previously written bullet of the section.
#[test]
fn changelog_opens_the_050_section_with_the_rollup_inside_it() {
    let changelog = load("CHANGELOG.md");
    let lines: Vec<&str> = changelog.lines().collect();
    let headings: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("## [0.5.0"))
        .map(|(at, _)| at)
        .collect();
    assert_eq!(
        headings.len(),
        1,
        "exactly one `## [0.5.0` heading may exist — a second, earlier heading would silently drop every older bullet from the generated release notes"
    );
    let start = headings[0];
    let end = lines[start + 1..]
        .iter()
        .position(|line| line.starts_with("## ["))
        .map(|offset| start + 1 + offset)
        .expect("the 0.5.0 section ends at the next ## [ heading");
    let section = &lines[start + 1..end];
    let text = section.join("\n");

    // The rollup block sits at the head of the section, above its first
    // `### Changed`, and names the era families the collapsed version-line
    // chain enumerated, plus this batch.
    let rollup_at = text
        .find(ROLLUP_MARKER)
        .expect("the section opens with the rollup block");
    let first_subsection = text.find("### Changed").expect("a ### subsection");
    assert!(
        rollup_at < first_subsection,
        "the rollup lives above the section's first subsection"
    );
    let rollup = &text[..first_subsection];
    for family in [
        "roles plan and contracts",
        "audit-defects",
        "relocation and roles-views surfaces",
        "cvd-precision",
        "go-facade honest-absence decision",
        "stdout destination contract",
        "views × markers roles contract",
        "doc-surface naming sweep",
        "machine-readable claim registry",
        "blind-2 and blind-4 follow-ups",
        "blind-5 follow-up",
    ] {
        assert!(
            rollup.contains(family),
            "the rollup summarizes the `{family}` era family:\n{rollup}"
        );
    }
    assert!(
        rollup.contains("released from this entry"),
        "the rollup states the release marker"
    );

    // Every previously present bullet survives byte-identical BELOW the
    // rollup — sampled anchors from the section's earlier prose.
    for anchor in [
        "- The claim registry now registers this plan and every wave-1..5 sentence it",
        "- A stale `report --check` now names the rendering it compared (blind5",
        "- Three help-surface relocations and one summary correction",
        "- The artefact dirt map is stated as one owner sentence at",
    ] {
        assert!(
            text.contains(anchor),
            "the pre-existing bullet {anchor:?} must survive byte-identical"
        );
        assert!(
            anchor.chars().count() > 0 && text[rollup_at..].contains(anchor),
            "the pre-existing bullet {anchor:?} lives below the rollup"
        );
    }
}

/// Scenario "version line collapses to one line with a release marker": the
/// line is pinned byte-exact, the value stays the tag number, none of the
/// old chain's era clauses survives anywhere in the file, and the lock pairs
/// the same value.
#[test]
fn version_line_is_one_release_marker_line() {
    let cargo = load("Cargo.toml");
    let lines: Vec<&str> = cargo.lines().collect();
    let version_lines: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("version = "))
        .collect();
    assert_eq!(version_lines.len(), 1, "the package version line is unique");
    assert_eq!(*version_lines[0], VERSION_LINE);

    // None of the collapsed chain's era clauses survives in the file; the
    // do-not-bump invariant survives exactly once, in the marker comment.
    for era_clause in [
        "as do the",
        "as does the",
        "unreleased tag",
        "auto-bump leaks",
        "roles plan (derivations",
        "blind4 US 01-03",
        "cvd US 01 + US 02",
    ] {
        assert!(
            !cargo.contains(era_clause),
            "the collapsed chain's era clause {era_clause:?} must not survive anywhere in Cargo.toml"
        );
    }
    assert_eq!(
        cargo
            .matches("do not bump past the tag until the release commit")
            .count(),
        1,
        "the do-not-bump clause survives exactly once, as the release marker"
    );

    let lock = load("Cargo.lock");
    assert!(
        lock.contains("name = \"rust-arch-test-kit\"\nversion = \"0.5.0\""),
        "Cargo.lock pairs the same 0.5.0 the manifest carries"
    );
}

/// Scenario "binary source stated where CI wiring looks": agents.md states
/// both sources — the build-from-source command and every asset name the
/// payload's release machinery emits — spells no URL and no hosting word
/// (the docs-tree URL-free invariant this surface keeps, which no
/// owner-qualified repo URL can ever break), and doc names and workflow
/// names cannot drift: both are compared against one pairing table.
#[test]
fn binary_source_statement_states_availability_and_spells_no_urls() {
    let agents = load("docs/archspec/agents.md");
    let lower = agents.to_lowercase();
    for forbidden in ["http", "://", ".com", "www.", "github"] {
        assert!(
            !lower.contains(forbidden),
            "the binary-source surface stays URL-free and hosting-neutral — the leak-guard-clean wording rule forbids {forbidden:?} here"
        );
    }
    assert!(
        agents.contains("cargo build --release --bin archspec")
            && agents.contains("target/release/archspec"),
        "the build-from-source command and binary location are stated"
    );
    assert!(
        agents.contains("published with each version tag")
            && agents.contains("release page for the tag"),
        "the release-tag availability is stated generically — the mechanism, not an owner URL"
    );
    assert!(
        agents.contains("`.sha256` sidecar"),
        "the sidecar convention is stated"
    );
    let release = load(".github/workflows/release.yml");
    for (doc_name, workflow_name) in RELEASE_ASSETS {
        assert!(
            agents.contains(doc_name),
            "the doc names the asset {doc_name:?} the release machinery emits"
        );
        let from_doc_dollar_brace = doc_name.replace("<version>", "${ver}");
        let from_doc_dollar = doc_name.replace("<version>", "$ver");
        assert!(
            release.contains(from_doc_dollar_brace.as_str())
                || release.contains(from_doc_dollar.as_str()),
            "release.yml emits the asset the doc names — {workflow_name:?} must appear, doc and workflow cite the same bytes"
        );
    }
}
