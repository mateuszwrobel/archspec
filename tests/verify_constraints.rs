mod common;

use common::{stderr, stdout};

/// A single-package crate named `app` with the given `lib.rs` and optional
/// extra module files, plus a spec declaring the `app` component so the only
/// divergences are the constraint findings under test.
fn app_fixture(extra_sources: &[(&str, &str)], spec_body: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    for (path, content) in extra_sources {
        fixture.write(path, content);
    }
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n\n{spec_body}"
        ),
    );
    fixture
}

fn assert_pass(output: &std::process::Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 (stderr: {})",
        stderr(output)
    );
    assert!(
        stdout(output).starts_with("ok: "),
        "pass confirmation on stdout:\n{}",
        stdout(output)
    );
}

fn assert_fail(output: &std::process::Output, expected_lines: &[&str]) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected non-zero exit (stderr: {})",
        stderr(output)
    );
    let out = stdout(output);
    assert!(
        out.contains("architecture.spec.toml does not match source model"),
        "report header on stdout:\n{out}"
    );
    for line in expected_lines {
        assert!(out.contains(line), "report must contain `{line}`:\n{out}");
    }
    assert!(
        stderr(output).is_empty(),
        "rule violations are report content, not stderr"
    );
}

// === public_api_allowlist (#16-#18) ===

fn public_api_fixture(lib: &str, allowed: &[&str]) -> common::Fixture {
    let allowed_toml: Vec<String> = allowed.iter().map(|a| format!("\"{a}\"")).collect();
    let spec = format!(
        "[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [{}]\n",
        allowed_toml.join(", ")
    );
    app_fixture(&[("src/lib.rs", lib)], &spec)
}

#[test]
fn verify_scenario_sixteen_allowlisted_exports_pass() {
    let fixture = public_api_fixture(
        "pub fn serve() {}\npub mod auth { pub fn x() {} }\n",
        &["auth", "serve"],
    );
    assert_pass(&fixture.run(&["verify"]));
}

#[test]
fn verify_scenario_seventeen_reports_unallowlisted_public_export() {
    let fixture = public_api_fixture(
        "pub fn serve() {}\npub mod auth { pub fn x() {} }\n",
        &["serve"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["public api leak: app exposes auth (not allowlisted)"],
    );
}

#[test]
fn verify_scenario_eighteen_reserved_root_plumbing_does_not_violate() {
    let fixture = public_api_fixture(
        "pub fn serve() {}\nfn helper() {}\nmod internal {}\n#[macro_export]\nmacro_rules! m {() => {}}\n",
        &["serve"],
    );
    assert_pass(&fixture.run(&["verify"]));
}

// === public_api_allowlist #51/#52: UNRESOLVABLE glob re-exports are unverifiable
// (amended contract: root globs that resolve to same-crate modules are
// enumerated and checked like named exports — see #71+ below) ===

#[test]
fn verify_scenario_51_reports_unresolvable_glob_reexport_as_unverifiable() {
    let fixture = public_api_fixture(
        "pub mod types { pub struct A {} }\npub use ghost::*;\npub fn serve() {}\n",
        &["A", "types", "serve"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["unverifiable glob export: app exposes ghost::*"],
    );
}

#[test]
fn verify_scenario_52_allowlisted_named_exports_not_reported_only_unresolvable_glob() {
    let fixture = public_api_fixture(
        "pub mod types { pub struct A {} }\npub use serde::*;\npub fn serve() {}\n",
        &["types", "serve"],
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected non-zero exit (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("unverifiable glob export: app exposes serde::*"),
        "report must contain the glob line:\n{out}"
    );
    assert!(
        !out.contains("public api leak"),
        "allowlisted named exports must not be reported:\n{out}"
    );
}

// === public_api_allowlist #71-#85: resolvable root globs are enumerated ===

/// Crate-root glob fixtures: the `app` crate with `lib.rs` plus extra files and
/// a `public_api_allowlist` constraint over `allowed`.
fn glob_fixture(lib: &str, extra: &[(&str, &str)], allowed: &[&str]) -> common::Fixture {
    let allowed_toml: Vec<String> = allowed.iter().map(|a| format!("\"{a}\"")).collect();
    let spec = format!(
        "[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [{}]\n",
        allowed_toml.join(", ")
    );
    let mut sources: Vec<(&str, &str)> = vec![("src/lib.rs", lib)];
    sources.extend_from_slice(extra);
    app_fixture(&sources, &spec)
}

// #71: a root glob naming a same-crate module is enumerated; every derived
// name allowlisted -> exit 0, no glob finding.
#[test]
fn verify_scenario_71_resolvable_root_glob_enumerated_against_allowed() {
    let fixture = glob_fixture(
        "mod types { pub struct Token; pub struct Config; }\npub use types::*;\n",
        &[],
        &["Config", "Token"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "a resolved glob produces no glob finding:\n{}",
        stdout(&output)
    );
}

// #72: a glob-derived export missing from `allowed` fails and names the item.
#[test]
fn verify_scenario_72_disallowed_glob_derived_export_fails() {
    let fixture = glob_fixture(
        "mod types { pub struct Token; pub struct Config; }\npub use types::*;\n",
        &[],
        &["Token"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["public api leak: app exposes Config (not allowlisted)"],
    );
}

// #73: allowlisted named exports and a resolvable glob coexist; the glob's
// names are checked individually, the named ones stay silent.
#[test]
fn verify_scenario_73_named_exports_and_resolvable_glob_coexist() {
    let fixture = glob_fixture(
        "pub fn serve() {}\nmod types { pub struct Token; }\npub use types::*;\n",
        &[],
        &["serve", "Token"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let leaking = glob_fixture(
        "pub fn serve() {}\nmod types { pub struct Token; }\npub use types::*;\n",
        &[],
        &["serve"],
    );
    assert_fail(
        &leaking.run(&["verify"]),
        &["public api leak: app exposes Token (not allowlisted)"],
    );
}

// #74: overlapping glob sets and a named export collapse to one check per name,
// and an unchanged source reruns byte-identically.
#[test]
fn verify_scenario_74_multiple_globs_and_duplicates_are_deterministic() {
    let lib = "mod a;\nmod b;\npub use a::*;\npub use b::*;\npub use a::Alpha;\n";
    let extra = [
        ("src/a.rs", "pub struct Alpha;\npub struct Shared;\n"),
        ("src/b.rs", "pub struct Beta;\npub struct Shared;\n"),
    ];
    let allowed_all = glob_fixture(lib, &extra, &["Alpha", "Beta", "Shared"]);
    assert_pass(&allowed_all.run(&["verify"]));

    let leaking = glob_fixture(lib, &extra, &["Alpha", "Beta"]);
    let first = leaking.run(&["verify"]);
    assert_fail(
        &first,
        &["public api leak: app exposes Shared (not allowlisted)"],
    );
    let out = stdout(&first);
    let duplicates = out.matches("app exposes Shared (not allowlisted)").count();
    assert_eq!(duplicates, 1, "each distinct name is checked once:\n{out}");
    let second = leaking.run(&["verify"]);
    assert_eq!(
        stdout(&first),
        stdout(&second),
        "unchanged source must rerun byte-identically"
    );
}

// #75: a recursive glob chain that cycles terminates; the union of the public
// items reachable through the chain is checked against `allowed`.
#[test]
fn verify_scenario_75_recursive_glob_chain_resolves_with_cycle_protection() {
    let fixture = glob_fixture(
        "mod a;\nmod b;\npub use a::*;\n",
        &[
            ("src/a.rs", "pub use crate::b::*;\npub struct InA;\n"),
            ("src/b.rs", "pub use crate::a::*;\npub struct InB;\n"),
        ],
        &["InA", "InB"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "a cycling chain resolves, no glob finding:\n{}",
        stdout(&output)
    );
}

// #76: an unresolvable link poisons the whole chain (weakest link wins).
#[test]
fn verify_scenario_76_unresolvable_link_poisons_chain() {
    let fixture = glob_fixture(
        "mod a;\npub use a::*;\n",
        &[("src/a.rs", "pub use serde::*;\npub struct InA;\n")],
        &["InA"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["unverifiable glob export: app exposes a::*"],
    );
    let out = stdout(&fixture.run(&["verify"]));
    assert!(
        !out.contains("public api leak"),
        "a poisoned chain reports no enumerated names:\n{out}"
    );
}

// #77: any cfg on the glob re-export declaration blocks resolution.
#[test]
fn verify_scenario_77_cfg_on_glob_declaration_blocks_resolution() {
    let fixture = glob_fixture(
        "mod types { pub struct Token; }\n#[cfg(all(feature = \"x\", unix))]\npub use types::*;\n",
        &[],
        &["Token"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["unverifiable glob export: app exposes types::*"],
    );
}

// #78: any cfg on the target module declaration blocks resolution.
#[test]
fn verify_scenario_78_cfg_on_target_module_blocks_resolution() {
    let fixture = glob_fixture(
        "#[cfg(test)]\nmod types;\npub use types::*;\n",
        &[("src/types.rs", "pub struct Token;\n")],
        &["Token"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["unverifiable glob export: app exposes types::*"],
    );
}

// #79: any cfg on a chain link blocks resolution, however deep.
#[test]
fn verify_scenario_79_cfg_on_chain_link_blocks_resolution() {
    let fixture = glob_fixture(
        "mod a;\nmod b;\npub use a::*;\n",
        &[
            ("src/a.rs", "pub use crate::b::*;\npub struct InA;\n"),
            ("src/b.rs", "#[cfg(feature = \"deep\")]\npub struct InB;\n"),
        ],
        &["InA", "InB"],
    );
    // cfg on a plain ITEM inside a reachable module does not block (names are
    // the union across configurations) — unlike cfg on a module/re-export decl.
    let output = fixture.run(&["verify"]);
    assert_pass(&output);

    let gated_link = glob_fixture(
        "mod a;\nmod b;\npub use a::*;\n",
        &[
            (
                "src/a.rs",
                "#[cfg(feature = \"deep\")]\npub use crate::b::*;\npub struct InA;\n",
            ),
            ("src/b.rs", "pub struct InB;\n"),
        ],
        &["InA", "InB"],
    );
    assert_fail(
        &gated_link.run(&["verify"]),
        &["unverifiable glob export: app exposes a::*"],
    );
}

// #80: `self::x::*`, `crate::x::*` and the crate-name prefix spell the same
// local glob; one deduplicated set is checked.
#[test]
fn verify_scenario_80_self_crate_and_name_prefixes_normalize() {
    let fixture = glob_fixture(
        "mod types { pub struct Token; }\npub use self::types::*;\npub use crate::types::*;\npub use app::types::*;\n",
        &[],
        &["Token"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let leaking = glob_fixture(
        "mod types { pub struct Token; }\npub use self::types::*;\npub use crate::types::*;\npub use app::types::*;\n",
        &[],
        &["Other"],
    );
    let first = leaking.run(&["verify"]);
    assert_fail(
        &first,
        &["public api leak: app exposes Token (not allowlisted)"],
    );
    let out = stdout(&first);
    assert_eq!(
        out.matches("app exposes Token (not allowlisted)").count(),
        1,
        "the three spellings yield one deduplicated set:\n{out}"
    );
}

// #80a: the crate-name prefix is dash/underscore-insensitive.
#[test]
fn verify_scenario_80a_crate_name_prefix_normalizes_dashes() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"voice-app-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod types { pub struct Token; }\npub use voice_app_lib::types::*;\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"voice-app-lib\"\nmatches = { units = [\"voice-app-lib\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"Token\"]\n",
    );
    assert_pass(&fixture.run(&["verify"]));
}

// #81: a first segment matching a declared external dependency is unresolvable.
#[test]
fn verify_scenario_81_external_dependency_prefix_is_unresolvable() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write("src/lib.rs", "pub fn serve() {}\npub use serde::*;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"serve\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["unverifiable glob export: app exposes serde::*"],
    );
}

// #82: a resolvable glob exporting zero public items fails closed, with and
// without `--strict`.
#[test]
fn verify_scenario_82_empty_resolvable_glob_fails_closed() {
    let fixture = glob_fixture(
        "mod empty_mod {}\npub use empty_mod::*;\n",
        &[],
        &["Anything"],
    );
    for args in [&["verify"][..], &["verify", "--strict"][..]] {
        let output = fixture.run(args);
        assert_ne!(
            output.status.code(),
            Some(0),
            "empty glob must fail without --strict too (stderr: {})",
            stderr(&output)
        );
        let out = stdout(&output);
        assert!(
            out.contains("empty glob export: app exposes empty_mod::*"),
            "report must name the empty glob:\n{out}"
        );
        assert!(
            !out.contains("unverifiable glob export"),
            "an empty glob is resolvable — not reported as unverifiable:\n{out}"
        );
        assert!(
            !out.contains("vacuous"),
            "a checked-but-empty glob is not a vacuous constraint:\n{out}"
        );
    }
}

// #83: emptiness reached through a chain also fails closed.
#[test]
fn verify_scenario_83_empty_glob_through_chain_fails_closed() {
    let fixture = glob_fixture(
        "mod a;\nmod b;\npub use a::*;\n",
        &[
            ("src/a.rs", "pub use crate::b::*;\n"),
            ("src/b.rs", "struct Hidden;\n"),
        ],
        &["Anything"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["empty glob export: app exposes a::*"],
    );
}

// #84: item-level cfg inside a reachable target module does not block
// enumeration (same union semantics as named exports).
#[test]
fn verify_scenario_84_item_level_cfg_does_not_block_enumeration() {
    let fixture = glob_fixture(
        "mod types { #[cfg(unix)] pub struct UnixOnly; pub struct Token; }\npub use types::*;\n",
        &[],
        &["Token", "UnixOnly"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "item-level cfg must not block enumeration:\n{}",
        stdout(&output)
    );
}

// #86: ownership precedence — a first segment that is BOTH a root module of the
// crate and a declared dependency resolves to the local module.
#[test]
fn verify_scenario_86_root_module_outranks_dependency_of_same_name() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod serde { pub struct Local; }\npub use serde::*;\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"Local\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "the local module wins over the dependency name:\n{}",
        stdout(&output)
    );
}

// #85: the motivating facade — `pub use core::types::*;` through a nested
// same-crate module path — resolves to the concrete items.
#[test]
fn verify_scenario_85_nested_same_crate_glob_path_resolves() {
    let fixture = glob_fixture(
        "mod core;\npub use core::types::*;\n",
        &[("src/core.rs", "pub mod types { pub struct Token; }\n")],
        &["Token"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let leaking = glob_fixture(
        "mod core;\npub use core::types::*;\n",
        &[("src/core.rs", "pub mod types { pub struct Token; }\n")],
        &["Nothing"],
    );
    assert_fail(
        &leaking.run(&["verify"]),
        &["public api leak: app exposes Token (not allowlisted)"],
    );
}

// #87: a `super` link inside a glob chain pops one segment — the chain
// resolves to m::other and its item is enumerated.
#[test]
fn verify_scenario_87_super_link_in_glob_chain_resolves() {
    let fixture = glob_fixture(
        "mod m;\npub use m::inner::*;\n",
        &[
            ("src/m/mod.rs", "mod inner;\nmod other;\n"),
            ("src/m/inner.rs", "pub use super::other::*;\n"),
            ("src/m/other.rs", "pub struct Real;\n"),
        ],
        &["Real"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "a super link resolves, no glob finding:\n{}",
        stdout(&output)
    );
    let leaking = glob_fixture(
        "mod m;\npub use m::inner::*;\n",
        &[
            ("src/m/mod.rs", "mod inner;\nmod other;\n"),
            ("src/m/inner.rs", "pub use super::other::*;\n"),
            ("src/m/other.rs", "pub struct Real;\n"),
        ],
        &["Nothing"],
    );
    assert_fail(
        &leaking.run(&["verify"]),
        &["public api leak: app exposes Real (not allowlisted)"],
    );
}

// #88: a private decoy `mod other` inside the chain's own module cannot
// shadow the super-resolved m/other.rs — the enumerated item is the real one.
#[test]
fn verify_scenario_88_decoy_module_cannot_shadow_super_resolved_target() {
    let fixture = glob_fixture(
        "mod m;\npub use m::inner::*;\n",
        &[
            ("src/m/mod.rs", "mod inner;\nmod other;\n"),
            ("src/m/inner.rs", "mod other;\npub use super::other::*;\n"),
            ("src/m/other.rs", "pub struct Real;\n"),
            ("src/m/inner/other.rs", "pub struct Decoy;\n"),
        ],
        &["Real"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("Decoy") && !stdout(&output).contains("glob"),
        "the decoy must not enter the surface:\n{}",
        stdout(&output)
    );
    let leaking = glob_fixture(
        "mod m;\npub use m::inner::*;\n",
        &[
            ("src/m/mod.rs", "mod inner;\nmod other;\n"),
            ("src/m/inner.rs", "mod other;\npub use super::other::*;\n"),
            ("src/m/other.rs", "pub struct Real;\n"),
            ("src/m/inner/other.rs", "pub struct Decoy;\n"),
        ],
        &["Nothing"],
    );
    let output = leaking.run(&["verify"]);
    assert_fail(
        &output,
        &["public api leak: app exposes Real (not allowlisted)"],
    );
    assert!(
        !stdout(&output).contains("Decoy"),
        "the decoy is not part of the public surface:\n{}",
        stdout(&output)
    );
}

// #89: a `super::super::` link pops one segment per token — two levels up.
#[test]
fn verify_scenario_89_double_super_chain_pops_one_segment_per_token() {
    let fixture = glob_fixture(
        "mod a;\nmod x;\npub use a::b::*;\n",
        &[
            ("src/a/mod.rs", "mod b;\n"),
            ("src/a/b.rs", "pub use super::super::x::*;\n"),
            ("src/x.rs", "pub struct Deep;\n"),
        ],
        &["Deep"],
    );
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "a double-super link resolves, no glob finding:\n{}",
        stdout(&output)
    );
}

// #92: a glob chain link declared with `#[path]` is located through its
// attributed file; that file's public surface is enumerated, not reported
// unverifiable.
#[test]
fn verify_scenario_92_path_link_in_glob_chain_resolves() {
    let lib = "mod wrapper;\nmod legacy;\npub use wrapper::inner::*;\n";
    let extra = [
        (
            "src/wrapper.rs",
            "#[path = \"renamed/inner_file.rs\"]\nmod inner;\n",
        ),
        (
            "src/renamed/inner_file.rs",
            "pub use crate::legacy::Real;\n",
        ),
        ("src/legacy.rs", "pub struct Real;\n"),
    ];
    let fixture = glob_fixture(lib, &extra, &["Real"]);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("glob"),
        "a resolved #[path] link produces no glob finding:\n{}",
        stdout(&output)
    );
    let leaking = glob_fixture(lib, &extra, &["Nothing"]);
    assert_fail(
        &leaking.run(&["verify"]),
        &["public api leak: app exposes Real (not allowlisted)"],
    );
}

// #93: a cfg_attr path link resolves through the first existing candidate in
// declaration order; the losing candidate's items never enter the surface.
#[test]
fn verify_scenario_93_cfg_attr_path_link_in_glob_chain_resolves() {
    let lib = "mod wrapper;\nmod legacy;\npub use wrapper::inner::*;\n";
    let extra = [
        (
            "src/wrapper.rs",
            "#[cfg_attr(unix, path = \"renamed/inner_file.rs\")]\n#[cfg_attr(windows, path = \"renamed/other_file.rs\")]\nmod inner;\n",
        ),
        ("src/renamed/inner_file.rs", "pub use crate::legacy::Real;\n"),
        ("src/renamed/other_file.rs", "pub struct Decoy;\n"),
        ("src/legacy.rs", "pub struct Real;\n"),
    ];
    let fixture = glob_fixture(lib, &extra, &["Real"]);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("Decoy") && !stdout(&output).contains("glob"),
        "the first existing candidate is analyzed, no glob finding:\n{}",
        stdout(&output)
    );
    let leaking = glob_fixture(lib, &extra, &["Nothing"]);
    let output = leaking.run(&["verify"]);
    assert_fail(
        &output,
        &["public api leak: app exposes Real (not allowlisted)"],
    );
    assert!(
        !stdout(&output).contains("Decoy"),
        "the losing candidate is not part of the public surface:\n{}",
        stdout(&output)
    );
}

// #94: a `#[path]` link with no target file fails closed — the whole glob is
// unverifiable and no guessed surface is enumerated.
#[test]
fn verify_scenario_94_path_link_with_missing_target_fails_closed() {
    let fixture = glob_fixture(
        "mod wrapper;\npub use wrapper::inner::*;\n",
        &[(
            "src/wrapper.rs",
            "#[path = \"renamed/missing_file.rs\"]\nmod inner;\n",
        )],
        &["Anything"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["unverifiable glob export: app exposes wrapper::inner::*"],
    );
    let out = stdout(&fixture.run(&["verify"]));
    assert!(
        !out.contains("public api leak"),
        "a poisoned chain reports no enumerated names:\n{out}"
    );
}

// === 22a/22b: crate-root exports scope to the unit root, not a
// `matches.modules` boundary ===

/// A single `app` crate whose spec declares only a module-tier boundary
/// (`matches.modules`, no `matches.units`). The unit is assigned to the
/// `config` boundary by module ownership; crate-root exports must NOT be
/// dumped onto that boundary.
fn module_tier_public_api_fixture(
    lib: &str,
    extra: &[(&str, &str)],
    allowed: &[&str],
) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", lib);
    for (path, content) in extra {
        fixture.write(path, content);
    }
    let allowed_toml: Vec<String> = allowed.iter().map(|a| format!("\"{a}\"")).collect();
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"config\"\nmatches = {{ modules = [\"auth\"] }}\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [{}]\n",
            allowed_toml.join(", ")
        ),
    );
    fixture
}

#[test]
fn verify_scenario_22a_module_tier_only_all_crate_exports_allowlisted_passes() {
    let fixture = module_tier_public_api_fixture(
        "pub fn serve() {}\npub mod auth;\n",
        &[("src/auth.rs", "pub fn x() {}\n")],
        &["serve", "auth"],
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(!out.contains("public api leak"), "no leak reported:\n{out}");
}

#[test]
fn verify_scenario_22b_module_tier_only_crate_export_leak_names_crate_not_module() {
    let fixture = module_tier_public_api_fixture(
        "pub fn serve() {}\npub mod auth;\n",
        &[("src/auth.rs", "pub fn x() {}\n")],
        &["serve"],
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["public api leak: app exposes auth (not allowlisted)"],
    );
}

#[test]
fn verify_scenario_22c_unit_matched_boundary_owns_crate_exports() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn serve() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"service\"\nmatches = { units = [\"app\"] }\n\n[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"internal\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["public api leak: service exposes serve (not allowlisted)"],
    );
}

// === forbid_external_crates (#19-#21) ===

fn external_crate_fixture(from: &[&str], forbid: &[&str]) -> common::Fixture {
    let from_toml: Vec<String> = from.iter().map(|f| format!("\"{f}\"")).collect();
    let forbid_toml: Vec<String> = forbid.iter().map(|f| format!("\"{f}\"")).collect();
    let spec = format!(
        "[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [{}]\nforbid = [{}]\n",
        from_toml.join(", "),
        forbid_toml.join(", ")
    );
    app_fixture(
        &[
            ("src/lib.rs", "pub mod core;\npub mod infra;\n"),
            ("src/core.rs", "use clap::Parser;\npub fn core() {}\n"),
            ("src/infra.rs", "use anyhow::Error;\npub fn infra() {}\n"),
        ],
        &spec,
    )
}

#[test]
fn verify_scenario_nineteen_no_listed_module_imports_forbidden_crate() {
    let fixture = external_crate_fixture(&["infra"], &["serde"]);
    assert_pass(&fixture.run(&["verify"]));
}

#[test]
fn verify_scenario_twenty_reports_listed_module_importing_forbidden_crate() {
    let fixture = external_crate_fixture(&["core"], &["clap"]);
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden external crate: core -> clap"],
    );
}

#[test]
fn verify_scenario_twenty_one_unlisted_module_importing_forbidden_crate_passes() {
    let fixture = external_crate_fixture(&["infra"], &["clap"]);
    assert_pass(&fixture.run(&["verify"]));
}

// === manifest_integrity (#22-#25) ===

fn manifest_fixture(manifest: &str, spec_body: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write("Cargo.toml", manifest);
    fixture.write("src/lib.rs", "pub fn app() {}\n");
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n\n{spec_body}"
        ),
    );
    fixture
}

#[test]
fn verify_scenario_twenty_two_manifest_matches_all_requirements() {
    let fixture = manifest_fixture(
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\n\n[features]\nfoo = []\ntelemetry = []\n",
        "[[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"serde\"]\nrequired_features = [\"telemetry\"]\n",
    );
    assert_pass(&fixture.run(&["verify"]));
}

#[test]
fn verify_scenario_twenty_three_reports_publish_mismatch() {
    let fixture = manifest_fixture(
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n",
        "[[constraint]]\ntype = \"manifest_integrity\"\nrequire_publish = true\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["manifest integrity: Cargo.toml publish=false (required true)"],
    );
}

#[test]
fn verify_scenario_twenty_four_reports_forbidden_dependency() {
    let fixture = manifest_fixture(
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nclap = \"4\"\n",
        "[[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"clap\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["manifest integrity: Cargo.toml has forbidden dependency clap"],
    );
}

#[test]
fn verify_scenario_twenty_five_reports_missing_required_feature() {
    let fixture = manifest_fixture(
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\nfoo = []\n",
        "[[constraint]]\ntype = \"manifest_integrity\"\nrequired_features = [\"telemetry\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["manifest integrity: Cargo.toml missing required feature telemetry"],
    );
}

// acceptance: at a workspace root (no `[package]`), manifest_integrity consults
// the per-unit manifest facts, so a forbidden dep a member declares is surfaced
// rather than being vacuous against the root's empty manifest.
#[test]
fn manifest_integrity_checks_per_unit_facts_at_workspace() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nclap = \"4\"\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"clap\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["manifest integrity: crates/billing/Cargo.toml has forbidden dependency clap"],
    );
}

// === manifest_integrity #53/#54/#55: each member manifest checked individually ===

fn workspace_manifest_fixture(members: &[(&str, &str)], spec_body: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    let member_list: Vec<String> = members
        .iter()
        .map(|(path, _)| format!("\"{path}\""))
        .collect();
    fixture.write(
        "Cargo.toml",
        &format!("[workspace]\nmembers = [{}]\n", member_list.join(", ")),
    );
    for (path, manifest) in members {
        fixture.write(&format!("{path}/Cargo.toml"), manifest);
        let name = path.rsplit('/').next().unwrap();
        fixture.write(
            &format!("{path}/src/lib.rs"),
            &format!("pub fn {name}() {{}}\n"),
        );
    }
    let module_decls: String = members
        .iter()
        .map(|(path, _)| {
            let name = path.rsplit('/').next().unwrap();
            format!("[[module]]\nname = \"{name}\"\nmatches = {{ units = [\"{name}\"] }}\n\n")
        })
        .collect();
    fixture.write(
        "architecture.spec.toml",
        &format!("[project]\nlanguage = \"rust\"\n\n{module_decls}{spec_body}"),
    );
    fixture
}

#[test]
fn verify_scenario_53_workspace_forbidden_dep_names_member_manifest() {
    let fixture = workspace_manifest_fixture(
        &[
            (
                "crates/auth",
                "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
            ),
            (
                "crates/billing",
                "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nclap = \"4\"\n",
            ),
        ],
        "[[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"clap\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["manifest integrity: crates/billing/Cargo.toml has forbidden dependency clap"],
    );
}

#[test]
fn verify_scenario_54_workspace_missing_feature_not_masked_by_sibling() {
    let fixture = workspace_manifest_fixture(
        &[
            (
                "crates/auth",
                "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\ntelemetry = []\n",
            ),
            (
                "crates/billing",
                "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            ),
        ],
        "[[constraint]]\ntype = \"manifest_integrity\"\nrequired_features = [\"telemetry\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["manifest integrity: crates/billing/Cargo.toml missing required feature telemetry"],
    );
}

#[test]
fn verify_scenario_55_workspace_all_members_satisfy_constraint_passes() {
    let fixture = workspace_manifest_fixture(
        &[
            (
                "crates/auth",
                "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\n\n[features]\ntelemetry = []\n",
            ),
            (
                "crates/billing",
                "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n\n[features]\ntelemetry = []\n",
            ),
        ],
        "[[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"clap\"]\nrequired_features = [\"telemetry\"]\n",
    );
    assert_pass(&fixture.run(&["verify"]));
}

// === feature_boundary (#26-#29) ===

fn feature_boundary_fixture(lib: &str, spec: &str) -> common::Fixture {
    app_fixture(
        &[
            ("src/lib.rs", lib),
            ("src/core.rs", "use crate::experimental::Thing;\n"),
            ("src/experimental.rs", "pub struct Thing;\n"),
        ],
        spec,
    )
}

#[test]
fn verify_scenario_twenty_six_gated_module_used_only_by_allowed_module() {
    let fixture = feature_boundary_fixture(
        "pub mod core;\n#[cfg(feature = \"experimental\")]\npub mod experimental;\n",
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"experimental\"\ngated_modules = [\"experimental\"]\nallowed_from = [\"core\"]\n",
    );
    assert_pass(&fixture.run(&["verify"]));
}

#[test]
fn verify_scenario_twenty_seven_reports_unallowed_module_depending_on_gated() {
    let fixture = feature_boundary_fixture(
        "pub mod core;\n#[cfg(feature = \"experimental\")]\npub mod experimental;\n",
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"experimental\"\ngated_modules = [\"experimental\"]\nallowed_from = [\"root\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["feature boundary: core depends on gated module experimental (feature: experimental)"],
    );
}

#[test]
fn verify_scenario_twenty_eight_reports_gated_module_missing_cfg() {
    let fixture = feature_boundary_fixture(
        "pub mod core;\npub mod experimental;\n",
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"experimental\"\ngated_modules = [\"experimental\"]\nallowed_from = [\"core\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &[
            "feature boundary: experimental.rs module 'experimental' not gated under cfg(feature = \"experimental\")",
        ],
    );
}

#[test]
fn verify_scenario_twenty_nine_reports_unknown_gated_module_pattern() {
    let fixture = feature_boundary_fixture(
        "pub mod core;\n#[cfg(feature = \"experimental\")]\npub mod experimental;\n",
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"experimental\"\ngated_modules = [\"nonexistent\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["feature boundary: gated module pattern 'nonexistent' matches no declared module"],
    );
}

// === feature_boundary #47: declared feature must match the module's actual
// gate; #48: a gated module covers its submodules (subtree boundary) ===

#[test]
fn verify_scenario_47_reports_feature_name_mismatch() {
    let fixture = app_fixture(
        &[
            ("src/lib.rs", "mod core;\n#[cfg(feature = \"uniffi\")]\npub mod experimental;\n"),
            ("src/core.rs", "use crate::experimental::Thing;\n"),
            ("src/experimental.rs", "pub struct Thing;\n"),
        ],
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"tauri\"\ngated_modules = [\"experimental\"]\nallowed_from = [\"core\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["feature boundary: experimental.rs module 'experimental' gated under feature \"uniffi\", not \"tauri\""],
    );
}

/// `tauri` is gated and contains a `commands` submodule. `external_mod` depends
/// on `tauri::commands` (inside the gated boundary) -> violation naming the
/// submodule. Meanwhile `tauri::commands -> tauri` (submodule into its own gated
/// parent) must NOT be a false violation.
#[test]
fn verify_scenario_48_gated_subtree_dep_violation_and_no_false_submodule() {
    let fixture = app_fixture(
        &[
            ("src/lib.rs", "mod external_mod;\n#[cfg(feature = \"tauri\")]\nmod tauri;\n"),
            ("src/tauri.rs", "pub mod commands;\n"),
            ("src/tauri/commands.rs", "pub fn run() {}\nuse crate::tauri;\n"),
            ("src/external_mod.rs", "use crate::tauri::commands::run;\n"),
        ],
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"tauri\"\ngated_modules = [\"tauri\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["feature boundary: external_mod depends on gated module tauri::commands (feature: tauri)"],
    );
    let out = stdout(&fixture.run(&["verify"]));
    assert!(
        !out.contains("commands depends on gated module tauri"),
        "submodule -> gated parent must not be a false violation:\n{out}"
    );
}

/// `allowed_from = ["root"]` (no gated module listed) still blocks a non-root
/// module depending on the gated subtree.
#[test]
fn verify_scenario_48_root_only_allowed_from_blocks_gated_subtree() {
    let fixture = app_fixture(
        &[
            ("src/lib.rs", "mod external_mod;\n#[cfg(feature = \"tauri\")]\nmod tauri;\n"),
            ("src/tauri.rs", "pub mod commands;\n"),
            ("src/tauri/commands.rs", "pub fn run() {}\n"),
            ("src/external_mod.rs", "use crate::tauri::commands::run;\n"),
        ],
        "[[constraint]]\ntype = \"feature_boundary\"\nfeature = \"tauri\"\ngated_modules = [\"tauri\"]\nallowed_from = [\"root\"]\n",
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["feature boundary: external_mod depends on gated module tauri::commands (feature: tauri)"],
    );
}

// === forbid_submodule_dependency (#30-#32) ===

fn submodule_fixture(common_body: &str, breakdown_body: &str, spec: &str) -> common::Fixture {
    app_fixture(
        &[
            ("src/lib.rs", "pub mod orchestration;\n"),
            (
                "src/orchestration.rs",
                "pub mod common;\npub mod breakdown;\npub mod orchestrator;\npub mod control_loop;\n",
            ),
            ("src/orchestration/common.rs", common_body),
            ("src/orchestration/breakdown.rs", breakdown_body),
            ("src/orchestration/orchestrator.rs", "pub fn orchestrator() {}\n"),
            ("src/orchestration/control_loop.rs", "pub struct Loop;\n"),
        ],
        spec,
    )
}

fn submodule_spec(from: &[&str], forbid: &[&str]) -> String {
    let from_toml: Vec<String> = from.iter().map(|f| format!("\"{f}\"")).collect();
    let forbid_toml: Vec<String> = forbid.iter().map(|f| format!("\"{f}\"")).collect();
    format!(
        "[[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"orchestration\"\nfrom = [{}]\nforbid = [{}]\n",
        from_toml.join(", "),
        forbid_toml.join(", ")
    )
}

#[test]
fn verify_scenario_thirty_no_forbidden_sibling_dependency() {
    let fixture = submodule_fixture(
        "pub fn common() {}\n",
        "use crate::orchestration::common::common;\npub fn breakdown() {}\n",
        &submodule_spec(&["breakdown"], &["control_loop"]),
    );
    assert_pass(&fixture.run(&["verify"]));
}

#[test]
fn verify_scenario_thirty_one_reports_forbidden_submodule_dependency() {
    let fixture = submodule_fixture(
        "use crate::orchestration::control_loop::Loop;\n",
        "pub fn breakdown() {}\n",
        &submodule_spec(&["common"], &["control_loop"]),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden submodule dependency: orchestration::common -> orchestration::control_loop"],
    );
}

#[test]
fn verify_scenario_thirty_two_unlisted_submodule_dependency_passes() {
    let fixture = submodule_fixture(
        "use crate::orchestration::orchestrator::orchestrator;\npub fn common() {}\n",
        "use crate::orchestration::control_loop::Loop;\npub fn breakdown() {}\n",
        &submodule_spec(&["common"], &["control_loop"]),
    );
    assert_pass(&fixture.run(&["verify"]));
}

/// `forbid_submodule_dependency` whose three keys resolve through the canonical
/// module-path matcher (workplan_engine_forbid_submodule_pattern_parity): the
/// same `parent` tree as `submodule_spec`, but with `parent`/`from`/`forbid`
/// written in the documented full, bare or unit-stripped form.
fn submodule_spec_paths(parent: &str, from: &[&str], forbid: &[&str]) -> String {
    let from_toml: Vec<String> = from.iter().map(|f| format!("\"{f}\"")).collect();
    let forbid_toml: Vec<String> = forbid.iter().map(|f| format!("\"{f}\"")).collect();
    format!(
        "[[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"{parent}\"\nfrom = [{}]\nforbid = [{}]\n",
        from_toml.join(", "),
        forbid_toml.join(", ")
    )
}

// The leak edge: `common` imports `control_loop`, one intra-parent submodule
// edge under `app::orchestration`.
const COMMON_LEAKS_CONTROL_LOOP: &str =
    "use crate::orchestration::control_loop::Loop;\npub fn common() {}\n";
const BREAKDOWN_CLEAN: &str = "pub fn breakdown() {}\n";

// Full-path `parent`/`from`/`forbid` (the manual's recommended precise form)
// engage and catch the leak, and the rule is not reported vacuous.
#[test]
fn verify_forbid_submodule_full_path_engages_and_catches() {
    let fixture = submodule_fixture(
        COMMON_LEAKS_CONTROL_LOOP,
        BREAKDOWN_CLEAN,
        &submodule_spec_paths(
            "app::orchestration",
            &["app::orchestration::common"],
            &["app::orchestration::control_loop"],
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_fail(
        &output,
        &["forbidden submodule dependency: orchestration::common -> orchestration::control_loop"],
    );
    let out = stdout(&output);
    assert!(
        !out.contains("vacuous constraint"),
        "an engaged rule must not read vacuous:\n{out}"
    );
}

// Bare-last-segment form (the pre-fix only working grammar) keeps working, so
// the correction is a superset, never a narrowing.
#[test]
fn verify_forbid_submodule_bare_tail_still_engages() {
    let fixture = submodule_fixture(
        COMMON_LEAKS_CONTROL_LOOP,
        BREAKDOWN_CLEAN,
        &submodule_spec_paths("app::orchestration", &["common"], &["control_loop"]),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden submodule dependency: orchestration::common -> orchestration::control_loop"],
    );
}

// Unit-stripped path (unit prefix omitted) engages too.
#[test]
fn verify_forbid_submodule_unit_stripped_engages() {
    let fixture = submodule_fixture(
        COMMON_LEAKS_CONTROL_LOOP,
        BREAKDOWN_CLEAN,
        &submodule_spec_paths(
            "orchestration",
            &["orchestration::common"],
            &["orchestration::control_loop"],
        ),
    );
    assert_fail(
        &fixture.run(&["verify"]),
        &["forbidden submodule dependency: orchestration::common -> orchestration::control_loop"],
    );
}

// A full-path spec over a tree with no matching forbidden edge passes: the
// widened resolution never invents findings for a clean tree.
#[test]
fn verify_forbid_submodule_full_path_no_false_positive() {
    let fixture = submodule_fixture(
        "use crate::orchestration::orchestrator::orchestrator;\npub fn common() {}\n",
        BREAKDOWN_CLEAN,
        &submodule_spec_paths(
            "app::orchestration",
            &["app::orchestration::common"],
            &["app::orchestration::control_loop"],
        ),
    );
    assert_pass(&fixture.run(&["verify"]));
}

// A `from` full path naming a real submodule that carries no intra-parent edge
// is genuinely vacuous — the warning wording is preserved, exit zero without
// `--strict`, failing under it.
#[test]
fn verify_forbid_submodule_vacuous_full_path_genuine_no_edge() {
    let fixture = submodule_fixture(
        "pub fn common() {}\n",
        "use crate::orchestration::control_loop::Loop;\npub fn breakdown() {}\n",
        &submodule_spec_paths(
            "app::orchestration",
            &["app::orchestration::common"],
            &["app::orchestration::control_loop"],
        ),
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a from pattern with no in-scope source edge is vacuous, not an error (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(
        out.contains("no intra-parent submodule edges to check"),
        "vacuous wording preserved:\n{out}"
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_ne!(
        strict.status.code(),
        Some(0),
        "--strict must promote the vacuous constraint"
    );
}

// === lib-internal groups: cycles inside one unit are visible to no_cycles
// (workplan 07: prefix subgroups split a crate-root boundary) ===

/// Single `kit` crate whose spec splits the lib into prefix groups
/// (`kit-graph`, `kit-rules`) under the crate-root boundary `kit`.
/// `graph_body` decides whether the graph side reaches back into rules.
fn lib_internal_group_fixture(
    graph_body: &str,
    rules_body: &str,
    graph_allowed: &[&str],
    rules_allowed: &[&str],
) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod graph;\npub mod rules;\n");
    fixture.write("src/graph.rs", graph_body);
    fixture.write("src/rules.rs", rules_body);
    let fmt = |list: &[&str]| {
        list.iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let graph_allowed_block = if graph_allowed.is_empty() {
        String::new()
    } else {
        format!("\n[module.allowed]\ndepend_on = [{}]\n", fmt(graph_allowed))
    };
    let rules_allowed_block = if rules_allowed.is_empty() {
        String::new()
    } else {
        format!("\n[module.allowed]\ndepend_on = [{}]\n", fmt(rules_allowed))
    };
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"kit\"\nmatches = {{ units = [\"kit\"] }}\n\n[[module]]\nname = \"kit-graph\"\nmatches = {{ modules = [\"kit::graph\"] }}\n{graph_allowed_block}\n[[module]]\nname = \"kit-rules\"\nmatches = {{ modules = [\"kit::rules\"] }}\n{rules_allowed_block}\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"kit-graph\", \"kit-rules\"]\n"
        ),
    );
    fixture
}

#[test]
fn verify_reports_internal_cycle_between_spec_grouped_lib_modules() {
    let fixture = lib_internal_group_fixture(
        "use crate::rules::Rule;\npub struct Node;\n",
        "use crate::graph::Node;\npub struct Rule;\n",
        &["kit-rules"],
        &["kit-graph"],
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "internal cycle inside one unit must fail (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("cycle: kit-graph -> kit-rules -> kit-graph")
            || out.contains("cycle: kit-rules -> kit-graph -> kit-rules"),
        "cycle naming the involved lib groups must be reported:\n{out}"
    );
}

#[test]
fn verify_lib_internal_groups_pass_when_dependency_is_one_way() {
    let fixture = lib_internal_group_fixture(
        "pub struct Node;\n",
        "use crate::graph::Node;\npub struct Rule;\n",
        &[],
        &["kit-graph"],
    );
    assert_pass(&fixture.run(&["verify"]));
}

/// #87 Forbidden-edge laundering through undeclared territory. A module
/// depends on a forbidden target only via a module the spec claims through no
/// boundary, so no direct boundary pair crosses the ban.
fn laundered_forbidden_fixture(hidden_claimed: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub mod a;\npub mod b;\npub mod hidden;\npub mod shell;\n",
    );
    fixture.write(
        "src/a.rs",
        "use crate::hidden::Hidden;\npub fn a() -> Hidden { crate::hidden::Hidden }\n",
    );
    fixture.write(
        "src/hidden.rs",
        "pub struct Hidden;\nimpl Hidden { pub fn go(&self) { crate::b::bee(); } }\n",
    );
    fixture.write("src/b.rs", "pub fn bee() {}\n");
    fixture.write("src/shell.rs", "pub fn nothing() {}\n");
    let shell_matches = if hidden_claimed {
        "modules = [\"app::shell\"], units = [\"app\"]"
    } else {
        "modules = [\"app::shell\"]"
    };
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = {{ modules = [\"app::a\"] }}\n\n[module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = {{ modules = [\"app::b\"] }}\n\n[[module]]\nname = \"shell\"\nmatches = {{ {shell_matches} }}\n\n[module.allowed]\ndepend_on = [\"b\"]\n"
        ),
    );
    fixture
}

#[test]
fn verify_fails_strict_when_forbidden_edge_is_laundered_via_intermediate_boundary() {
    let fixture = laundered_forbidden_fixture(true);
    // Without --strict the finding is listed as a warning but tolerated.
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "warnings are tolerated without --strict (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("warning: laundered forbidden edge: a -> b via shell"),
        "warning listed on stdout:\n{}",
        stdout(&output)
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_fail(&output, &["laundered forbidden edge: a -> b via shell"]);
}

#[test]
fn verify_fails_strict_on_module_edge_endpoint_owned_by_no_boundary() {
    let fixture = laundered_forbidden_fixture(false);
    let output = fixture.run(&["verify", "--strict"]);
    assert_fail(&output, &["unowned module edge endpoint: app::hidden"]);
}

/// #88 A `#[path]`-located module file is part of the module's source: its
/// imports must reach `forbid_external_crates` like any other file variant.
#[test]
fn verify_forbidden_external_crates_see_path_attribute_modules() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub mod visible;\n#[path = \"misc/thing.rs\"]\npub mod hidden;\n",
    );
    fixture.write("src/visible.rs", "pub fn visible() {}\n");
    fixture.write(
        "src/misc/thing.rs",
        "use serde::Serialize;\npub fn hidden() {}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::hidden\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_fail(&output, &["forbidden external crate: hidden -> serde"]);
    assert!(
        !stdout(&output).contains("unresolved module file"),
        "a resolved #[path] target must not be reported unresolved:\n{}",
        stdout(&output)
    );
}

/// #90 A `#[cfg_attr(..., path = "...")]` declaration names candidate locations
/// for the module file: the first candidate that exists is analyzed like a
/// `#[path]` target, its dependencies reach `forbid_external_crates`, and no
/// `unresolved module file` warning is emitted for the declaration.
#[test]
fn verify_forbidden_external_crates_see_cfg_attr_path_modules() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub mod visible;\n#[cfg_attr(unix, path = \"os/unix.rs\")]\n#[cfg_attr(windows, path = \"os/windows.rs\")]\npub mod os;\n",
    );
    fixture.write("src/visible.rs", "pub fn visible() {}\n");
    fixture.write(
        "src/os/unix.rs",
        "use serde::Serialize;\npub fn platform() {}\n",
    );
    fixture.write("src/os/windows.rs", "pub fn platform() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::os\"]\nforbid = [\"serde\"]\n",
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_fail(&output, &["forbidden external crate: os -> serde"]);
    assert!(
        !stdout(&output).contains("unresolved module file"),
        "a resolvable cfg_attr path candidate must not be reported unresolved:\n{}",
        stdout(&output)
    );
}

/// #91 A `#[cfg_attr(..., path = "...")]` declaration whose candidates all miss
/// stays invisible to every check if skipped silently, so `verify` reports it
/// like any other unresolved declaration and `--strict` fails it.
#[test]
fn verify_reports_cfg_attr_path_declarations_that_resolve_to_no_file() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "pub mod visible;\n#[cfg_attr(unix, path = \"os/unix.rs\")]\n#[cfg_attr(windows, path = \"os/windows.rs\")]\npub mod os;\n",
    );
    fixture.write("src/visible.rs", "pub fn visible() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    assert_fail(
        &fixture.run(&["verify", "--strict"]),
        &["unresolved module file: app::os"],
    );
}

/// #89 A file-backed `mod X;` that resolves to no known file variant would
/// silently hide the module's contents from every check; `verify` reports it
/// instead, and `--strict` fails it.
#[test]
fn verify_reports_module_declarations_that_resolve_to_no_file() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod present;\npub mod ghost;\n");
    fixture.write("src/present.rs", "pub fn present() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    assert_fail(
        &fixture.run(&["verify", "--strict"]),
        &["unresolved module file: app::ghost"],
    );
}

// === forbidden-edge laundering: legal layering vs conduit territory ===
// spec.md "Forbidden bans are transitive on the boundary graph": a banned
// pair unreachable through purely declared boundaries but reachable once
// unit-fallback hops join is laundering; paths bridged entirely by
// explicitly claimed boundaries are legal layering (facade, composition
// root) and the skip branch in check_forbidden_laundering must not report
// them — nor mask a conduit path coexisting with a declared one.

/// `mixed = false`: `a -> x -> b` where `x` is claimed by its own explicit
/// modules boundary (purely declared layering). `mixed = true`: additionally
/// `a -> w -> b` where `w` is claimed only through the unit fallback of the
/// catch-all `conduit` boundary (laundering conduit).
fn layering_forbidden_fixture(mixed: bool) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        if mixed {
            "pub mod a;\npub mod b;\npub mod x;\npub mod w;\n"
        } else {
            "pub mod a;\npub mod b;\npub mod x;\n"
        },
    );
    fixture.write(
        "src/a.rs",
        if mixed {
            "use crate::x::X;\nuse crate::w::W;\npub fn a(x: &X, w: &W) { x.go(); w.go(); }\n"
        } else {
            "use crate::x::X;\npub fn a(x: &X) { x.go(); }\n"
        },
    );
    fixture.write(
        "src/x.rs",
        "use crate::b::bee;\npub struct X;\nimpl X { pub fn go(&self) { bee(); } }\n",
    );
    if mixed {
        fixture.write(
            "src/w.rs",
            "use crate::b::bee;\npub struct W;\nimpl W { pub fn go(&self) { bee(); } }\n",
        );
    }
    fixture.write("src/b.rs", "pub fn bee() {}\n");
    let a_deps = if mixed {
        "[\"x\", \"conduit\"]"
    } else {
        "[\"x\"]"
    };
    let conduit_module = if mixed {
        "\n[[module]]\nname = \"conduit\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"b\"]\n"
    } else {
        ""
    };
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = {{ modules = [\"app::a\"] }}\n\n[module.allowed]\ndepend_on = {a_deps}\nforbidden = [\"b\"]\n\n[[module]]\nname = \"x\"\nmatches = {{ modules = [\"app::x\"] }}\n\n[module.allowed]\ndepend_on = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = {{ modules = [\"app::b\"] }}\n{conduit_module}"
        ),
    );
    fixture
}

/// #95 The skip branch: the ban `a -> b` is bridged entirely by explicitly
/// claimed boundaries (`x`), so the path is legal layering — no laundering
/// warning, no violation, exit 0.
#[test]
fn verify_scenario_95_purely_declared_intermediate_is_legal_layering_not_laundering() {
    let fixture = layering_forbidden_fixture(false);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("laundered forbidden edge"),
        "a path bridged purely by declared boundaries must not warn:\n{}",
        stdout(&output)
    );
}

/// #96 A purely declared path coexisting with a conduit path must not be
/// masked by the skip branch: laundering is still reported.
#[test]
fn verify_scenario_96_declared_path_alongside_conduit_path_still_reports_laundering() {
    let fixture = layering_forbidden_fixture(true);
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "laundering stays a warning without --strict (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("warning: laundered forbidden edge: a -> b via"),
        "the skip branch must not mask a conduit path coexisting with a declared one:\n{}",
        stdout(&output)
    );
}

/// #97 `--strict` variants: the legal-layering fixture exits 0, the mixed
/// fixture fails on the laundering warning.
#[test]
fn verify_scenario_97_strict_fails_mixed_conduit_but_not_pure_layering() {
    let output = layering_forbidden_fixture(false).run(&["verify", "--strict"]);
    assert_pass(&output);
    let output = layering_forbidden_fixture(true).run(&["verify", "--strict"]);
    assert_fail(&output, &["laundered forbidden edge"]);
}

// === publication-only root facade (structural, f21) ===
// A crate root that DEFINES NOTHING (only `mod` declarations and re-exports)
// is a publication-only facade: an internal module importing an item through
// the root (`use crate::X;` for a root re-export) is a violation regardless of
// declared `depend_on` — the umbrella would launder any ban. Activation is
// structural: the rule is off the moment the root file defines an item
// (cfg(test)-gated definitions do not count — tests are not production
// surface). Suppression path: canonicalize the import.

/// Facade crate builder: `app` with the given sources and a spec that names
/// the crate root `umbrella` (unit fallback) plus the extra module entries.
fn facade_fixture(
    sources: &[(&str, &str)],
    root_deps: &[&str],
    extra_modules: &str,
) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    for (path, content) in sources {
        fixture.write(path, content);
    }
    let deps: Vec<String> = root_deps.iter().map(|d| format!("\"{d}\"")).collect();
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = {{ units = [\"app\"] }}\n\n[module.allowed]\ndepend_on = [{}]\n{extra_modules}",
            deps.join(", ")
        ),
    );
    fixture
}

fn facade_a_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("src/lib.rs", "mod engine;\npub use engine::Thing;\n"),
        (
            "src/engine.rs",
            "pub struct Thing;\nuse crate::Thing;\npub fn make() -> Thing { Thing }\n",
        ),
    ]
}

/// #98a (fixture A): internal module imports a root re-export; the root is
/// DECLARED in `depend_on` and must still not legalize the edge.
#[test]
fn verify_scenario_98_internal_import_of_facade_root_reexport_is_violation() {
    let engine_module =
        "\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n";
    let fixture = facade_fixture(&facade_a_sources(), &["engine"], engine_module);
    let output = fixture.run(&["verify"]);
    assert_fail(&output, &["facade dependency: app::engine -> app"]);
    let out = stdout(&output);
    assert!(
        !out.contains("disallowed cross-component dependency"),
        "the declared depend_on keeps the pair legal for THAT check; only the facade rule fires:\n{out}"
    );
    assert!(
        !out.contains("missing edge:"),
        "the declared engine -> umbrella edge is present in the source:\n{out}"
    );
}

/// #98b (fixture B): the root DEFINES an item used internally -> rule
/// inactive -> identical edge shape passes.
#[test]
fn verify_scenario_98b_root_defining_items_deactivates_facade_rule() {
    let sources = vec![
        (
            "src/lib.rs",
            "mod engine;\npub use engine::Thing;\npub struct Shared;\n",
        ),
        (
            "src/engine.rs",
            "pub struct Thing;\nuse crate::Shared;\npub fn build(s: &Shared) -> Thing { let _ = s; Thing }\n",
        ),
    ];
    let engine_module =
        "\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n";
    let fixture = facade_fixture(&sources, &["engine"], engine_module);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("facade"),
        "a defining root is not a facade:\n{}",
        stdout(&output)
    );
}

/// #98c (fixture C): canonical internal import, root re-exports still exist
/// -> clean exit 0 (facade active, but no internal->root edge).
#[test]
fn verify_scenario_98c_canonical_import_stays_clean_under_active_facade_rule() {
    let sources = vec![
        (
            "src/lib.rs",
            "mod engine;\nmod client;\npub use engine::Thing;\npub use client::run;\n",
        ),
        ("src/engine.rs", "pub struct Thing;\n"),
        (
            "src/client.rs",
            "use crate::engine::Thing;\npub fn run() -> Thing { Thing }\n",
        ),
    ];
    let modules = "\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[[module]]\nname = \"client\"\nmatches = { modules = [\"app::client\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n";
    let fixture = facade_fixture(&sources, &["engine", "client"], modules);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    assert!(
        !stdout(&output).contains("facade"),
        "canonical imports never touch the facade root:\n{}",
        stdout(&output)
    );
}

/// #98d (fixture D): `--strict` parity (error-level, byte-identical verdict)
/// and composition with the laundering check: a ban can no longer be routed
/// through the umbrella without an always-visible facade error.
#[test]
fn verify_scenario_98d_strict_parity_and_laundering_composition() {
    let engine_module =
        "\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n";
    let fixture = facade_fixture(&facade_a_sources(), &["engine"], engine_module);
    let plain = fixture.run(&["verify"]);
    let strict = fixture.run(&["verify", "--strict"]);
    assert_fail(&plain, &["facade dependency: app::engine -> app"]);
    assert_fail(&strict, &["facade dependency: app::engine -> app"]);
    assert_eq!(
        stdout(&plain),
        stdout(&strict),
        "an error-level category is unaffected by --strict"
    );

    // Laundering composition: ban a -> b; a imports b's item THROUGH the
    // facade root. The internal->root edge is now a hard violation, and the
    // existing laundering check still reports the ban routed through the
    // umbrella — a silent launder is gone.
    let laundering = common::Fixture::new();
    laundering.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    laundering.write("src/lib.rs", "mod a;\nmod b;\npub use b::bee;\n");
    laundering.write("src/a.rs", "use crate::bee;\npub fn a() { bee(); }\n");
    laundering.write("src/b.rs", "pub fn bee() {}\n");
    laundering.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"b\"]\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"app::a\"] }\n\n[module.allowed]\nforbidden = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"app::b\"] }\n",
    );
    let output = laundering.run(&["verify"]);
    assert_fail(&output, &["facade dependency: app::a -> app"]);
    let out = stdout(&output);
    assert!(
        out.contains("laundered forbidden edge: a -> b via umbrella"),
        "the facade rule composes with the existing laundering check:\n{out}"
    );
}

/// #98e: cfg(test)-gated definitions at the root do NOT deactivate the rule —
/// tests are not production surface (`is_cfg_test` semantics reused).
#[test]
fn verify_scenario_98e_cfg_test_definitions_do_not_deactivate_facade_rule() {
    let mut sources = vec![(
        "src/lib.rs",
        "mod engine;\npub use engine::Thing;\n#[cfg(test)]\nstruct TestHelper;\n",
    )];
    sources.extend(facade_a_sources().into_iter().skip(1));
    let engine_module =
        "\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n";
    let fixture = facade_fixture(&sources, &["engine"], engine_module);
    let output = fixture.run(&["verify"]);
    assert_fail(&output, &["facade dependency: app::engine -> app"]);
}

// === external_free (zero-dependency purity guard) ===

/// A single `app` crate with a pure `core` module and a `ui` module importing
/// serde, plus the given constraint body.
fn purity_app(extra_constraint: &str, core_src: &str) -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\nserde_json = \"1\"\n",
    );
    fixture.write("src/lib.rs", "pub mod core;\npub mod ui;\n");
    fixture.write("src/core.rs", core_src);
    fixture.write("src/ui.rs", "use serde::Serialize;\npub fn ui() {}\n");
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n\n{extra_constraint}"
        ),
    );
    fixture
}

const PURE_CORE: &str = "pub struct Order;\nimpl Order { pub fn id(&self) -> u32 { 0 } }\n";
const CONTAMINATED_CORE: &str =
    "use serde::Serialize;\nuse serde_json;\npub struct Order;\npub fn dump() {}\n";

fn purity_constraint(from: &str, severity: &str) -> String {
    let severity_line = if severity.is_empty() {
        String::new()
    } else {
        format!("severity = \"{severity}\"\n")
    };
    format!("[[constraint]]\ntype = \"external_free\"\nfrom = [\"{from}\"]\n{severity_line}")
}

#[test]
fn external_free_pure_module_passes_without_vacuity_noise() {
    let fixture = purity_app(&purity_constraint("app::core", ""), PURE_CORE);
    let output = fixture.run(&["verify"]);
    assert_pass(&output);
    let out = stdout(&output);
    assert!(
        !out.contains("vacuous"),
        "a pure matched module is a real check, never a vacuous warning:\n{out}"
    );
    assert!(
        out.contains("1 constraints checked"),
        "the pure constraint counts as checked:\n{out}"
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_pass(&strict);
}

#[test]
fn external_free_contamination_fails_and_names_packages() {
    let fixture = purity_app(&purity_constraint("app::core", ""), CONTAMINATED_CORE);
    let output = fixture.run(&["verify"]);
    assert_fail(
        &output,
        &["not external free: core imports serde, serde_json"],
    );
}

#[test]
fn external_free_dead_pattern_is_vacuous_with_distinct_diagnostic() {
    let fixture = purity_app(&purity_constraint("app::domian", ""), PURE_CORE);
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a vacuous finding must not fail without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("vacuous constraint:"), "report:\n{out}");
    assert!(out.contains("external_free"), "report:\n{out}");
    assert!(
        out.contains(
            "'from' pattern \"app::domian\" matches no module or unit present in the model"
        ),
        "the diagnostic must name the dead pattern and say presence, not externals:\n{out}"
    );
    assert!(
        !out.contains("matches source model"),
        "vacuous-only run must not claim a match:\n{out}"
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "--strict must promote the vacuous finding"
    );
}

#[test]
fn external_free_warning_severity_tolerated_without_strict_fails_with_it() {
    let fixture = purity_app(
        &purity_constraint("app::core", "warning"),
        CONTAMINATED_CORE,
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a warning-severity finding exits 0 without --strict (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("warning: not external free: core imports serde, serde_json"),
        "finding listed as warning:\n{out}"
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "--strict promotes the warning"
    );
    assert!(
        stdout(&strict).contains("not external free: core imports serde, serde_json"),
        "promoted line keeps the finding:\n{}",
        stdout(&strict)
    );
}

#[test]
fn external_free_unit_pattern_monitors_whole_unit_subtree() {
    let fixture = purity_app(&purity_constraint("app", ""), PURE_CORE);
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a unit-naming pattern monitors every external attributed anywhere to the unit (stderr: {})",
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("not external free: ui imports serde"),
        "the attribution site must be named, not the unit:\n{}",
        stdout(&output)
    );
}

#[test]
fn external_free_and_forbid_external_findings_coexist() {
    let spec = format!(
        "{}\n[[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"app::core\"]\nforbid = [\"serde\"]\n",
        purity_constraint("app::core", "")
    );
    let fixture = purity_app(&spec, CONTAMINATED_CORE);
    let output = fixture.run(&["verify"]);
    assert_fail(&output, &[]);
    let out = stdout(&output);
    assert!(
        out.contains("forbidden external crate: core -> serde"),
        "the forbid rule reports:\n{out}"
    );
    assert!(
        out.contains("not external free: core imports serde, serde_json"),
        "the purity guard reports alongside it:\n{out}"
    );
}

#[test]
fn external_free_on_a_module_that_owns_externals_fails_immediately() {
    let fixture = purity_app(&purity_constraint("app::ui", ""), PURE_CORE);
    let output = fixture.run(&["verify"]);
    assert_fail(&output, &["not external free: ui imports serde"]);
}

#[test]
fn external_free_without_from_field_is_a_schema_error() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn x() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[constraint]]\ntype = \"external_free\"\n",
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(output.status.code(), Some(0));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains("`from`"),
        "registration through the required-fields table must name the missing key:\n{err}"
    );
}

/// A single-module go tree carries no module tier: the matched element is the
/// package unit itself and purity engages at the unit tier the driver emits.
#[test]
fn external_free_pure_go_package_unit_passes_non_vacuously() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write("core/core.go", "package core\n\nfunc Order() {}\n");
    fixture.write(
        "app/app.go",
        "package app\n\nimport \"modernc.org/sqlite\"\n\nfunc Run() { _ = sqlite.Open }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[module]]\nname = \"core\"\nmatches = { units = [\"example.com/demo/core\"] }\n\n\
         [[module]]\nname = \"app\"\nmatches = { units = [\"example.com/demo/app\"] }\n\n\
         [[constraint]]\ntype = \"external_free\"\nfrom = [\"example.com/demo/core\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "pure go package unit must verify clean (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "pass confirmation:\n{out}");
    assert!(
        !out.contains("vacuous"),
        "unit presence engages the constraint — no vacuous warning:\n{out}"
    );
}

#[test]
fn external_free_go_contamination_names_the_package() {
    let fixture = common::Fixture::new();
    fixture.write("go.mod", "module example.com/demo\ngo 1.21\n");
    fixture.write(
        "core/core.go",
        "package core\n\nimport \"modernc.org/sqlite\"\n\nfunc Order() { _ = sqlite.Open }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[module]]\nname = \"core\"\nmatches = { units = [\"example.com/demo/core\"] }\n\n\
         [[constraint]]\ntype = \"external_free\"\nfrom = [\"example.com/demo/core\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a contaminated core package must fail (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("not external free:")
            && out.contains("example.com/demo/core")
            && out.contains("modernc.org/sqlite"),
        "the finding must name the package unit and the offending dependency:\n{out}"
    );
}

#[test]
fn external_free_csharp_pure_module_passes_and_contamination_names_package() {
    let fixture = common::Fixture::new();
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  <ItemGroup>\n    <PackageReference Include=\"Newtonsoft.Json\" />\n  </ItemGroup>\n</Project>\n",
    );
    fixture.write(
        "App/Core.cs",
        "namespace App.Core;\npublic class Order { }\n",
    );
    fixture.write(
        "App/Ui.cs",
        "using Newtonsoft.Json;\nnamespace App.Ui;\npublic class Ui { public void Render() { Newtonsoft.Json.JsonConvert.Null.ToString(); } }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"App\"] }\n\n[[constraint]]\ntype = \"external_free\"\nfrom = [\"App::Core\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a pure c# module verifies clean (stderr: {})",
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        !out.contains("vacuous"),
        "a present pure module engages the guard:\n{out}"
    );

    fixture.write(
        "App/Core.cs",
        "using Newtonsoft.Json;\nnamespace App.Core;\npublic class Order { public string Tag() { return Newtonsoft.Json.JsonConvert.Null.ToString(); } }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "an EF-style contamination must fail"
    );
    let out = stdout(&output);
    assert!(
        out.contains("not external free: Core imports Newtonsoft.Json"),
        "the finding names the module and the package:\n{out}"
    );
}

/// Scenario 98f (workplan archspec_roles, US 05; de-vacuated by US 05b): the
/// bin-root exemption of the facade rule is stated by the composition ROLE,
/// not the `::main` name. The scan records `<unit>::main` with the
/// composition role (the `mod wiring;` edge out of main is the wiring fact)
/// and the BIN unit is a DECLARED boundary consuming its lib by name — the
/// verdict is asserted by exit code, not a substring filter (the old model's
/// only findings were the undeclared bin unit's `unexpected component` noise,
/// vacuous for the exemption). The rust module tier never crosses units (a
/// `use app::Thing` records the unit-tier edge only — proven by the scan
/// model), so the edge INTO a facade root recorded under a composition-role
/// module — the exemption branch itself — is pinned by scenario 98h on c#,
/// where such edges exist.
#[test]
fn verify_scenario_98f_bin_root_exemption_is_stated_by_the_composition_role() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod engine;\npub use engine::Thing;\n");
    fixture.write("src/engine.rs", "pub struct Thing;\n");
    fixture.write(
        "src/wiring.rs",
        "pub struct Bound;\npub use crate::Thing;\n",
    );
    fixture.write(
        "src/main.rs",
        "use app::Thing;\nmod wiring;\nfn main() { let _ = Thing; let _ = wiring::Bound; }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[[module]]\nname = \"bin\"\nmatches = { units = [\"app-bin\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n",
    );
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&fixture.run(&["scan"]))).expect("scan json");
    assert_eq!(
        model["roles"]["app-bin::main"].as_str(),
        Some("composition"),
        "the bin target (unit `app-bin`) carries the composition role: {:?}",
        model["roles"]
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_pass(&output);
}

/// Scenario 98g (workplan archspec_roles, US 05): the exemption follows the
/// role, not the name — a LIB module that happens to be named `main` and
/// imports through the root re-export is an internal module consuming the
/// facade like any other, and is reported.
#[test]
fn verify_scenario_98g_lib_module_named_main_is_internal_under_facade_root() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "mod engine;\npub use engine::Thing;\n#[path = \"worker.rs\"]\nmod main;\n",
    );
    fixture.write("src/engine.rs", "pub struct Thing;\n");
    fixture.write("src/worker.rs", "use crate::Thing;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a lib module named `main` without the composition role is internal:\n{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("facade dependency: app::main -> app"),
        "the name alone must not exempt a module:\n{}",
        stdout(&output)
    );
}

// === the composition exemption is hop-level (workplan archspec_roles,
// US 05b hardening) ===
// The boundary-level bearer whitelist (landed in US 05) sanctioned a WHOLE
// boundary that owned any composition-role path: one planted bin main plus a
// catch-all boundary claiming the bin's unit laundered every ban routed
// through that boundary (X2). The exemption is now keyed on the roles map's
// PATHS: a hop is sanctioned exactly when its source module carries the
// composition role — irrelevant which boundary claims the path's unit. The
// stage-1/2/3 attribution ladder is gone with the boundary-level whitelist.

/// X2 adversarial (code review of US 05): the laundered fixture (#87, strict
/// exit 1) gains a planted `src/main.rs` (composition at `app-bin::main`) and
/// the `shell` catch-all claims both units (`units = ["app", "app-bin"]`), so
/// `shell` becomes the boundary that owns the composition path — exactly the
/// shape the old whitelist whitelisted whole. The banned route `a -> shell -> b`
/// rides hops attributed from `app::a` and `app::hidden`: neither path carries
/// the composition role, so the laundering must survive the planted main.
/// RED on the pre-fix tree (exit 0), green with the hop-level exemption.
#[test]
fn verify_scenario_99_laundered_ban_survives_a_composition_role_on_the_catchall_boundary() {
    let fixture = laundered_forbidden_fixture(true);
    fixture.write("src/main.rs", "mod glue;\nfn main() { glue::nothing(); }\n");
    fixture.write("src/glue.rs", "pub fn nothing() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { modules = [\"app::a\"] }\n\n[module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"b\"]\n\n[[module]]\nname = \"b\"\nmatches = { modules = [\"app::b\"] }\n\n[[module]]\nname = \"shell\"\nmatches = { modules = [\"app::shell\"], units = [\"app\", \"app-bin\"] }\n\n[module.allowed]\ndepend_on = [\"b\"]\n",
    );
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&fixture.run(&["scan"]))).expect("scan json");
    assert_eq!(
        model["roles"]["app-bin::main"].as_str(),
        Some("composition"),
        "the planted main wiring is the composition the catch-all boundary claims: {:?}",
        model["roles"]
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_fail(
        &output,
        &["laundered forbidden edge: a -> b via shell"],
    );
}

/// The roles map of a model as a plain sorted list of (model path, role).
fn role_pairs(model: &serde_json::Value) -> Vec<(String, String)> {
    model["roles"]
        .as_object()
        .expect("roles must be an object")
        .iter()
        .map(|(path, role)| (path.clone(), role.as_str().expect("role").to_string()))
        .collect()
}

/// The c# facade-plus-composition-root pair: unit `App` publishes through its
/// root namespace (using-facts-only, the facade role); unit `Api` is an entry
/// project referencing it whose `Program.cs` wires the umbrella BY ITS ROOT
/// NAMESPACE (`using App;` — the edge INTO the facade root recorded under the
/// entry root module). `registrations` decides whether that root also carries
/// DI registration calls (composition at `Api`) or only usings (facade).
fn csharp_facade_and_root(registrations: bool) -> common::Fixture {
    let csproj = |references: &[&str]| {
        let mut text = String::from(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
        );
        if !references.is_empty() {
            text.push_str("  <ItemGroup>\n");
            for reference in references {
                text.push_str(&format!(
                    "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
                ));
            }
            text.push_str("  </ItemGroup>\n");
        }
        text.push_str("</Project>\n");
        text
    };
    let fixture = common::Fixture::new();
    fixture.write("App/App.csproj", &csproj(&[]));
    fixture.write("App/Root.cs", "namespace App\n{\n    using App.Engine;\n}\n");
    fixture.write(
        "App/Engine.cs",
        "namespace App.Engine;\npublic class Engine { }\n",
    );
    fixture.write("Api/Api.csproj", &csproj(&["App"]));
    fixture.write(
        "Api/Program.cs",
        &format!(
            "using App;\nvar builder = WebApplication.CreateBuilder(args);\n{}\nvar app = builder.Build();\napp.Run();\n",
            if registrations {
                "builder.Services.AddScoped<IOrderService, OrderService>();"
            } else {
                "// no registrations"
            }
        ),
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { modules = [\"App\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"App::Engine\"] }\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"Api\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n",
    );
    fixture
}

/// Scenario 98h (US 05b de-vacuation of 98f): the facade rule's
/// Role::Composition exemption branch executes in a REAL fixture. The rust
/// module tier never crosses units (a bin consumes its lib through the
/// unit-tier edge — proven by 98f's model), so an edge INTO a facade root
/// recorded under a composition-role module exists on c# drivers: the
/// composition root wires the umbrella by root namespace and the verdict is
/// asserted by exit code — no substring tolerance.
#[test]
fn verify_scenario_98h_csharp_composition_root_wiring_the_facade_root_is_exempt() {
    let fixture = csharp_facade_and_root(true);
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&fixture.run(&["scan"]))).expect("scan json");
    assert_eq!(
        role_pairs(&model),
        vec![
            ("Api".to_string(), "composition".to_string()),
            ("App".to_string(), "facade".to_string()),
        ],
        "one roles map states both the composition root and the facade root: {:?}",
        model["roles"]
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_pass(&output);
}

/// Scenario 98i (companion of 98h): the same shape WITHOUT the composition
/// role — no registration calls, the entry root states facade — makes the edge
/// INTO the facade root pure consumption, and the facade finding FIRES.
#[test]
fn verify_scenario_98i_root_wiring_into_facade_without_composition_role_is_reported() {
    let fixture = csharp_facade_and_root(false);
    let output = fixture.run(&["verify", "--strict"]);
    assert_fail(&output, &["facade dependency: Api -> App"]);
}

/// The multi-segment bridge (solution `Company` + `Company.App`): the
/// composition root is the multi-segment unit root `Company::App`, which no
/// `matches.modules` pattern and no unit rollup reaches — rollup splits at the
/// FIRST `::` and attributes every `Company.*` path to the neighbour boundary
/// claiming the `Company` unit. The composition boundary claims its unit
/// through `units` alone.
fn csharp_multi_segment_bridge(backdoor: bool) -> common::Fixture {
    let csproj = |references: &[&str]| {
        let mut text = String::from(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
        );
        if !references.is_empty() {
            text.push_str("  <ItemGroup>\n");
            for reference in references {
                text.push_str(&format!(
                    "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
                ));
            }
            text.push_str("  </ItemGroup>\n");
        }
        text.push_str("</Project>\n");
        text
    };
    let fixture = common::Fixture::new();
    fixture.write("Company/Company.csproj", &csproj(&[]));
    fixture.write(
        "Company/OrderService.cs",
        "namespace Company.Services;\nusing Company.Data;\npublic class OrderService { private readonly IOrderRepository _repo; public OrderService(IOrderRepository repo) { _repo = repo; } }\n",
    );
    fixture.write(
        "Company/OrderRepository.cs",
        "namespace Company.Data;\npublic interface IOrderRepository { }\npublic class OrderRepository : IOrderRepository { }\n",
    );
    if backdoor {
        fixture.write(
            "Company/Ghost.cs",
            "namespace Company.Ghost;\nusing Company.Services;\npublic class Ghost { }\n",
        );
    }
    fixture.write("Company.App/Company.App.csproj", &csproj(&["Company"]));
    fixture.write(
        "Company.App/Program.cs",
        "using Company.Services;\nvar builder = WebApplication.CreateBuilder(args);\nbuilder.Services.AddScoped<IOrderService, OrderService>();\nvar app = builder.Build();\napp.Run();\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"comp\"\nmatches = { units = [\"Company.App\"] }\n\n[module.allowed]\ndepend_on = [\"lib\"]\n\n[[module]]\nname = \"lib\"\nmatches = { units = [\"Company\"] }\n\n[module.allowed]\nforbidden = [\"data\"]\n\n[[module]]\nname = \"services\"\nmatches = { modules = [\"Company::Services\"] }\n\n[module.allowed]\ndepend_on = [\"data\"]\n\n[[module]]\nname = \"data\"\nmatches = { modules = [\"Company::Data\"] }\n",
    );
    fixture
}

/// Scenario 98j (US 05b): a ban on the boundary whose territory is only
/// attributed by UNIT membership (`Company`, next to the composition unit)
/// verifies clean when nothing routes through it — the composition root's
/// wiring hop is sanctioned by its path and no hop is attributed to the ban.
/// A backdoor planted in that same territory (`Company::Ghost` using the
/// composition-wired `Company::Services`) launders the ban through the
/// sanctioned hop and fails `--strict`: the hop-level exemption sanctions the
/// composition hop itself, never the whole attributed neighbourhood. On the
/// old ladder the composition-adjacent boundary was whitelisted wholesale and
/// the backdoor vanished (exit 0 — red pre-fix).
#[test]
fn verify_scenario_98j_multisegment_composition_hop_sanctioned_sibling_not_whitelisted() {
    let wiring = csharp_multi_segment_bridge(false);
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&wiring.run(&["scan"]))).expect("scan json");
    assert_eq!(
        role_pairs(&model),
        vec![("Company::App".to_string(), "composition".to_string())],
        "the multi-segment unit root carries the composition role at its PATH: {:?}",
        model["roles"]
    );
    let output = wiring.run(&["verify", "--strict"]);
    assert_pass(&output);
    let backdoor = csharp_multi_segment_bridge(true);
    let output = backdoor.run(&["verify", "--strict"]);
    assert_fail(&output, &["laundered forbidden edge"]);
}

/// Scenario 100 (US 05b): multi-root roles-map sanity — one workspace, two
/// publication-only lib roots (`app` and `core`, each a facade) and a bin
/// (`cli`) whose main root wires modules (one composition at `cli::main`).
/// The hop-level exemption reads the roles map as a SET OF PATHS: several
/// facades plus a composition must not confuse the map consumers, and honest
/// wiring across two umbrellas verifies clean.
#[test]
fn verify_scenario_100_roles_map_with_two_facades_and_a_composition_root_verifies_clean() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\", \"core\", \"cli\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "app/src/lib.rs",
        "mod engine;\npub use engine::Thing;\n",
    );
    fixture.write("app/src/engine.rs", "pub struct Thing;\n");
    fixture.write(
        "core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("core/src/lib.rs", "mod db;\npub use db::Pool;\n");
    fixture.write("core/src/db.rs", "pub struct Pool;\n");
    fixture.write(
        "cli/Cargo.toml",
        "[package]\nname = \"cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\napp = { path = \"../app\" }\ncore = { path = \"../core\" }\n",
    );
    fixture.write("cli/src/main.rs", "mod glue;\nfn main() { glue::run(); }\n");
    fixture.write(
        "cli/src/glue.rs",
        "pub fn run() {\n    let _ = app::Thing;\n    let _ = core::Pool;\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app_umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[[module]]\nname = \"core_umbrella\"\nmatches = { units = [\"core\"] }\n\n[module.allowed]\ndepend_on = [\"db\"]\n\n[[module]]\nname = \"db\"\nmatches = { modules = [\"core::db\"] }\n\n[[module]]\nname = \"cli\"\nmatches = { units = [\"cli\"] }\n\n[module.allowed]\ndepend_on = [\"app_umbrella\", \"core_umbrella\"]\n",
    );
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&fixture.run(&["scan"]))).expect("scan json");
    assert_eq!(
        role_pairs(&model),
        vec![
            ("app".to_string(), "facade".to_string()),
            ("cli::main".to_string(), "composition".to_string()),
            ("core".to_string(), "facade".to_string()),
        ],
        "two facade roots and the composition root coexist in one roles map: {:?}",
        model["roles"]
    );
    let output = fixture.run(&["verify", "--strict"]);
    assert_pass(&output);
}

/// ADR-018 regression net: the unit-ownership lookup is an ALLOWANCE lookup
/// only. A declared `depend_on` entry is still satisfied by a unit-tier pair
/// — a namespace-attributed edge whose unit attribution the spec never
/// connects by a reference (no unit edge) leaves the declared dependency
/// missing: the missing-edge check resolves pairs as before, so the fix
/// cannot quietly re-key what counts as a stated dependency.
#[test]
fn namespace_attribution_does_not_satisfy_a_declared_depend_on() {
    let fixture = common::Fixture::new();
    let csproj = |references: &[&str]| {
        let mut text = String::from(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
        );
        if !references.is_empty() {
            text.push_str("  <ItemGroup>\n");
            for reference in references {
                text.push_str(&format!(
                    "    <ProjectReference Include=\"..\\{reference}\\{reference}.csproj\" />\n"
                ));
            }
            text.push_str("  </ItemGroup>\n");
        }
        text.push_str("</Project>\n");
        text
    };
    fixture.write("Shop.Api/Shop.Api.csproj", &csproj(&["Shop.Data"]));
    fixture.write(
        "Shop.Api/Program.cs",
        "using Shop.Data;\nbuilder.Services.AddScoped<IOrderService, OrderService>();\nvar builder = WebApplication.CreateBuilder(args);\nbuilder.Build().Run();\n",
    );
    fixture.write("Shop.Data/Shop.Data.csproj", &csproj(&[]));
    fixture.write(
        "Shop.Data/OrderService.cs",
        "namespace Shop.Data;\npublic class OrderService { }\npublic class IOrderService { }\n",
    );
    fixture.write("Shop.Domain/Shop.Domain.csproj", &csproj(&[]));
    fixture.write(
        "Shop.Domain/Contracts.cs",
        "namespace Shop.Domain;\npublic interface IOrderRepository { }\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Shop.Api\"\nmatches = { units = [\"Shop.Api\"] }\nallowed = { depend_on = [\"Shop.Data\", \"Shop.Domain\"] }\n\n[[module]]\nname = \"Shop.Data\"\nmatches = { units = [\"Shop.Data\"] }\n\n[[module]]\nname = \"Shop.Domain\"\nmatches = { units = [\"Shop.Domain\"] }\n\n[[module]]\nname = \"Shop::Api\"\nmatches = { modules = [\"Shop::Api\"] }\n",
    );
    let output = fixture.run(&["verify"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a declared dependency with no stated edge must fail verify (stdout: {})",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("missing edge: Shop.Api -> Shop.Domain"),
        "the missing-edge check stays unit-tier: {}",
        stdout(&output)
    );
}
