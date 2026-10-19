//! Declarative driver capability table: one row per (language, fact) stating
//! what that language's extraction path emits, at which granularity, or
//! `not-emitted`. Single
//! source of truth read at runtime by `verify` (inert-rule notes) and
//! `doctor` (driver capability reported apart from toolchain availability).
//!
//! The table must mirror current per-driver behavior; every cell was verified
//! against live driver probes before codifying. Adding or changing a fact
//! requires updating the feature-matrix probes and this table in the same
//! change (same convention as the leak-guard rule). No logic lives here:
//! consumers decide what an emission value means; this module only states it.

use crate::archspec::language::Language;

/// The driver emits the fact at full granularity.
pub const GRANULAR: &str = "granular";
/// The driver never emits the fact; rules that require it can never fire for
/// the language.
pub const NOT_EMITTED: &str = "not-emitted";
/// Test evidence is applied at the file tier (`*_test.go` dropped at scan)
/// with no serialized module-path fact.
pub const FILE_TIER_ONLY: &str = "file tier only (*_test.go excluded at scan)";
/// Root module declarations exist as vertices but carry no gating facts:
/// the ts driver declares every scanned file of the package and emits no
/// feature-gating marks, so a `feature_boundary` constraint can never match
/// a gated declaration for the language.
pub const DECLARATIONS_NO_GATING: &str = "declarations, no gating facts";

/// File-grained dependency facts: rows identified by source FILE rather than
/// by claim or module path — the ts driver emits them natively (its module
/// identity IS the file path, so its `module_edges` rows carry the file
/// identity), the rust driver serializes them in the additive `file_edges`
/// rows with its lexical kind vocabulary. The go and csharp rows read
/// `not-emitted`: their edges are claim/module-grained and serialization
/// there is a named flip change, not today's fact.
pub const FACT_FILE_GRAINED_DEPENDENCY_FACTS: &str = "file-grained-dependency-facts";

/// `facade` role entries the driver derives into the model's serialized
/// roles map (which modules are publication facades).
pub const FACT_ROLE_FACADE: &str = "role-facade";
/// `composition` role entries the driver derives into the model's serialized
/// roles map (which modules are composition roots).
pub const FACT_ROLE_COMPOSITION: &str = "role-composition";
/// Soft module paths and module-level edges of the model.
pub const FACT_MODULE_TIER: &str = "module-tier";
/// Symbol lists carried on module edges.
pub const FACT_SYMBOLS: &str = "symbols";
/// Top-level module declarations of unit roots (feature-gating facts).
pub const FACT_ROOT_MODULE_DECLARATIONS: &str = "root-module-declarations";
/// Which sources/units/modules are architecture-excluded test scaffolding.
pub const FACT_TEST_TIER: &str = "test-tier";
/// External packages attributed to modules and the project.
pub const FACT_EXTERNAL_PACKAGES: &str = "external-packages";
/// Public API surface facts consumed by `public_api_allowlist` and by the
/// `contract.expose` symbol arm: rust enumerates crate-root exports (the
/// `root_public_exports` family — the allowlist's source; the expose symbol
/// arm has no module-tier source yet and states its vacuity per the named
/// follow-up plan), the C# driver as module-attributed public types
/// (`module_public_types`, with root-key rows reserved to the allowlist); a
/// driver with neither source states `not-emitted` and can never engage the
/// constraint for real.
pub const FACT_PUBLIC_API_SURFACE: &str = "public-api-surface";

/// The report-only orphan census (workplan
/// workplan_archspec_unreferenced_census, D8): per-file `orphan file:`
/// lines plus the `orphan file census:` summary on the ts scan report
/// surface. Drivers without the arm read `not-emitted` — an honest
/// asymmetry, each flipping its row only behind its own pilot
/// measurement (the ts-first precedent).
pub const FACT_ORPHAN_CENSUS: &str = "orphan-census";

/// The structural facade-role check of `verify` (rule name unchanged; the
/// fact source is the model's roles map).
pub const RULE_FACADE_DEPENDENCY: &str = "facade dependency";
/// The ban-routing (conduit) check of `verify`.
pub const RULE_LAUNDERED_FORBIDDEN_EDGE: &str = "laundered forbidden edge";

/// The claim-tier scope: today's adjudication, byte-unchanged (ADR-023).
pub const SCOPE_CLAIMS: &str = "claims";
/// The union scope: the claim tier plus the file-grained adjacency
/// (ADR-023). A row at this scope requires the capability
/// `file-grained-dependency-facts` to read `granular` — the weld test of
/// `capability.rs` pins the two tables together.
pub const SCOPE_BOTH: &str = "both";

/// The per-language deep-cycle posture (ts-driver design record D7,
/// ADR-023): the `(scope, severity)` a `no_cycles` row with no `scope`
/// derives from. Declared data beside the capability table, projected by
/// `capability matrix`. The rust row reads `both, error` since the named
/// flip change landed the counted portfolio cleanup, the model
/// serialization of file facts, and the zero-count re-probe (D11).
pub const POSTURES: &[(Language, &str, &str)] = &[
    (Language::Rust, SCOPE_BOTH, "error"),
    (Language::Csharp, SCOPE_CLAIMS, "error"),
    (Language::Go, SCOPE_CLAIMS, "error"),
    (Language::Ts, SCOPE_BOTH, "error"),
    // The python driver serializes file-identified `module_edges` rows from
    // day one (python driver record P2), so its posture is the ts D7 shape.
    (Language::Python, SCOPE_BOTH, "error"),
    // The kotlin driver serializes `file_edges` file facts beside gradle-path
    // `module_edges` rows from day one (kotlin driver record K-P2/K-P3), so
    // its posture is the same both shape.
    (Language::Kotlin, SCOPE_BOTH, "error"),
];

/// The posture declared for one language: the `(scope, severity)` a
/// `no_cycles` row with no `scope` derives from (D7).
pub fn posture(language: Language) -> (&'static str, &'static str) {
    POSTURES
        .iter()
        .find(|row| row.0 == language)
        .map(|row| (row.1, row.2))
        .unwrap_or((SCOPE_CLAIMS, "error"))
}

/// One row per (language, fact), stating what that language's sole extraction
/// path emits (capability-layer data, no engine introspection; every cell was
/// verified against live driver probes). Ordered by fact, then language.
pub const CAPABILITIES: &[(Language, &str, &str)] = &[
    (Language::Rust, FACT_ROLE_FACADE, GRANULAR),
    (Language::Csharp, FACT_ROLE_FACADE, GRANULAR),
    (Language::Go, FACT_ROLE_FACADE, NOT_EMITTED),
    (Language::Ts, FACT_ROLE_FACADE, GRANULAR),
    // The python driver (python driver record P2/P4): roles derive from no
    // python fact (no publication-fade heuristic), so both role rows read
    // not-emitted; module/symbols/test/external/file rows carry the
    // file-identified `module_edges` the driver serializes from day one.
    (Language::Python, FACT_ROLE_FACADE, NOT_EMITTED),
    (Language::Kotlin, FACT_ROLE_FACADE, NOT_EMITTED),
    (Language::Rust, FACT_ROLE_COMPOSITION, GRANULAR),
    (Language::Csharp, FACT_ROLE_COMPOSITION, GRANULAR),
    (Language::Go, FACT_ROLE_COMPOSITION, GRANULAR),
    (Language::Ts, FACT_ROLE_COMPOSITION, NOT_EMITTED),
    (Language::Python, FACT_ROLE_COMPOSITION, NOT_EMITTED),
    (Language::Kotlin, FACT_ROLE_COMPOSITION, NOT_EMITTED),
    (Language::Rust, FACT_MODULE_TIER, GRANULAR),
    (Language::Csharp, FACT_MODULE_TIER, GRANULAR),
    (Language::Go, FACT_MODULE_TIER, GRANULAR),
    (Language::Ts, FACT_MODULE_TIER, GRANULAR),
    (Language::Python, FACT_MODULE_TIER, GRANULAR),
    (Language::Kotlin, FACT_MODULE_TIER, GRANULAR),
    (Language::Rust, FACT_SYMBOLS, GRANULAR),
    (Language::Csharp, FACT_SYMBOLS, GRANULAR),
    (Language::Go, FACT_SYMBOLS, GRANULAR),
    (Language::Ts, FACT_SYMBOLS, GRANULAR),
    (Language::Python, FACT_SYMBOLS, GRANULAR),
    (Language::Kotlin, FACT_SYMBOLS, GRANULAR),
    (Language::Rust, FACT_ROOT_MODULE_DECLARATIONS, GRANULAR),
    (Language::Csharp, FACT_ROOT_MODULE_DECLARATIONS, NOT_EMITTED),
    (Language::Go, FACT_ROOT_MODULE_DECLARATIONS, NOT_EMITTED),
    (Language::Ts, FACT_ROOT_MODULE_DECLARATIONS, DECLARATIONS_NO_GATING),
    (Language::Python, FACT_ROOT_MODULE_DECLARATIONS, DECLARATIONS_NO_GATING),
    // The kotlin driver states its modules as units and gradle-path module
    // rows, never as per-file declarations (files are vertices, packages are
    // identity — K-P3), so this row reads not-emitted exactly like go.
    (Language::Kotlin, FACT_ROOT_MODULE_DECLARATIONS, NOT_EMITTED),
    (Language::Rust, FACT_TEST_TIER, GRANULAR),
    (Language::Csharp, FACT_TEST_TIER, GRANULAR),
    (Language::Go, FACT_TEST_TIER, FILE_TIER_ONLY),
    (Language::Ts, FACT_TEST_TIER, GRANULAR),
    (Language::Python, FACT_TEST_TIER, GRANULAR),
    (Language::Kotlin, FACT_TEST_TIER, GRANULAR),
    (Language::Rust, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Csharp, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Go, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Ts, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Python, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Kotlin, FACT_EXTERNAL_PACKAGES, GRANULAR),
    // Public API surface (the C# gap closed here): both fact sources were
    // live-probed — rust's root-export enumeration by the existing suite,
    // csharp's module public types by the csharp_public_api probes (issue
    // repro tree and the 3-segment corporate tree); go populates no public
    // API fact at all and its allowlist stays a fact-emptiness vacuity.
    (Language::Rust, FACT_PUBLIC_API_SURFACE, GRANULAR),
    (Language::Csharp, FACT_PUBLIC_API_SURFACE, GRANULAR),
    (Language::Go, FACT_PUBLIC_API_SURFACE, NOT_EMITTED),
    (Language::Ts, FACT_PUBLIC_API_SURFACE, GRANULAR),
    // The python driver populates no public-API fact: `__all__` is a
    // symbol-level fact, PARKED with the ts parity line (python driver
    // record, Open Questions), so the row reads not-emitted exactly like go.
    (Language::Python, FACT_PUBLIC_API_SURFACE, NOT_EMITTED),
    (Language::Kotlin, FACT_PUBLIC_API_SURFACE, NOT_EMITTED),
    // File-grained dependency facts (ts-driver design record, C6/D11): the ts
    // arm's edges are file-identified by construction; the rust driver
    // serializes file-grained facts in the model (`file_edges` rows) and its
    // row reads granular — the D11 serialization precondition. What stays a
    // later named flip change is the rust POSTURE move to `both` scope (and
    // the OQ-1 cleanup plus fresh re-probe beside it); go and csharp drivers
    // serialize no file facts and their rows stay not-emitted.
    (Language::Rust, FACT_FILE_GRAINED_DEPENDENCY_FACTS, GRANULAR),
    (Language::Csharp, FACT_FILE_GRAINED_DEPENDENCY_FACTS, NOT_EMITTED),
    (Language::Go, FACT_FILE_GRAINED_DEPENDENCY_FACTS, NOT_EMITTED),
    (Language::Ts, FACT_FILE_GRAINED_DEPENDENCY_FACTS, GRANULAR),
    // The python driver keys `module_edges` rows by file identity from its
    // first commit (python driver record P2), so its row is granular on
    // landing and its posture is the ts `both` shape.
    (Language::Python, FACT_FILE_GRAINED_DEPENDENCY_FACTS, GRANULAR),
    (Language::Kotlin, FACT_FILE_GRAINED_DEPENDENCY_FACTS, GRANULAR),
    // The orphan census (census workplan, D8): the ts driver states the
    // class on the scan report surface (proven by the ts_driver census
    // scenarios); every driver without the arm reads not-emitted — the
    // row flips only behind that driver's own pilot measurement.
    (Language::Rust, FACT_ORPHAN_CENSUS, NOT_EMITTED),
    (Language::Csharp, FACT_ORPHAN_CENSUS, NOT_EMITTED),
    (Language::Go, FACT_ORPHAN_CENSUS, NOT_EMITTED),
    (Language::Ts, FACT_ORPHAN_CENSUS, GRANULAR),
    (Language::Python, FACT_ORPHAN_CENSUS, NOT_EMITTED),
    (Language::Kotlin, FACT_ORPHAN_CENSUS, NOT_EMITTED),
];

/// Checks that consume a capability fact: `(rule, required fact)`. A rule is
/// inert for a language when its fact is `NOT_EMITTED`.
pub const RULES: &[(&str, &str)] = &[
    (RULE_FACADE_DEPENDENCY, FACT_ROLE_FACADE),
    (RULE_LAUNDERED_FORBIDDEN_EDGE, FACT_MODULE_TIER),
];

/// The emission value the table states for one (language, fact), or `None`
/// for an unknown fact/language pair.
pub fn emission(language: Language, fact: &str) -> Option<&'static str> {
    CAPABILITIES
        .iter()
        .find(|(row_language, row_fact, _)| {
            *row_language == language && *row_fact == fact
        })
        .map(|(_, _, emission)| *emission)
}

/// True when the driver emits the fact at full granularity (the row says
/// `GRANULAR`). Conditionals (`FILE_TIER_ONLY`) and
/// not-emitted rows yield `false`: on a plain fixture tree the fact is only
/// there when the table says the driver always puts it there. Wired into
/// `archspec capability granular` (the machine query behind the prose-drift
/// guards).
pub fn emits_granular(language: Language, fact: &str) -> bool {
    emission(language, fact) == Some(GRANULAR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_states_every_fact() {
        for language in [
            Language::Rust,
            Language::Csharp,
            Language::Go,
            Language::Ts,
            Language::Python,
        ] {
            let rows: Vec<(&str, &str)> = CAPABILITIES
                .iter()
                .filter(|(row_language, _, _)| *row_language == language)
                .map(|(_, fact, emission)| (*fact, *emission))
                .collect();
            assert_eq!(rows.len(), 10, "{language:?} must state all ten facts");
            for (fact, value) in rows {
                assert_eq!(
                    emission(language, fact),
                    Some(value),
                    "lookup must agree with the table row"
                );
            }
        }
    }

    #[test]
    fn role_facade_rows_match_the_derivations() {
        // Written from live probes after the derivations shipped (roles US
        // 01-03): rust derives facade from the defines-nothing root predicate,
        // csharp from the using-only umbrella root module, and the go driver
        // derives no facade from its own facts.
        assert_eq!(emission(Language::Rust, FACT_ROLE_FACADE), Some(GRANULAR));
        assert_eq!(
            emission(Language::Csharp, FACT_ROLE_FACADE),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Go, FACT_ROLE_FACADE),
            Some(NOT_EMITTED)
        );
        // The ts arm derives facade roles from its own facts (C4): a
        // package-root `index.ts` whose every row is an export-from
        // republication enters the roles map as `facade` — the ts
        // counterpart of rust's defines-nothing root predicate.
        assert_eq!(emission(Language::Ts, FACT_ROLE_FACADE), Some(GRANULAR));
    }

    #[test]
    fn role_composition_fact_is_granular_for_the_deriving_drivers() {
        // Every deriving driver reads composition from its own facts: rust
        // from the wiring bin main root, csharp from DI-registration
        // entrypoints, go from the package-main module. The ts arm carries a
        // roles map it never fills (a single package root has no composition
        // root to derive), so its row states not-emitted.
        assert_eq!(
            emission(Language::Rust, FACT_ROLE_COMPOSITION),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Csharp, FACT_ROLE_COMPOSITION),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Go, FACT_ROLE_COMPOSITION),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Ts, FACT_ROLE_COMPOSITION),
            Some(NOT_EMITTED)
        );
    }

    #[test]
    fn retired_root_facade_fact_answers_nothing() {
        // The `root-facade` fact retired with the serde-skipped field it
        // described (roles US 04): no alias row, and no language answers it.
        for language in [Language::Rust, Language::Csharp, Language::Go] {
            assert_eq!(
                emission(language, "root-facade"),
                None,
                "the retired fact must not answer for {language:?}"
            );
        }
    }

    #[test]
    fn facade_rule_pairs_onto_the_role_facade_fact() {
        let fact = RULES
            .iter()
            .find(|(rule, _)| *rule == RULE_FACADE_DEPENDENCY)
            .map(|(_, fact)| *fact);
        assert_eq!(
            fact,
            Some("role-facade"),
            "the facade rule must consume the role fact, not a retired one"
        );
    }

    #[test]
    fn module_tier_fact_is_granular_for_every_language() {
        assert_eq!(emission(Language::Rust, FACT_MODULE_TIER), Some(GRANULAR));
        assert_eq!(
            emission(Language::Csharp, FACT_MODULE_TIER),
            Some(GRANULAR)
        );
        assert_eq!(emission(Language::Go, FACT_MODULE_TIER), Some(GRANULAR));
        // The ts module identity is the file path — the tier exists at the
        // finest granularity the table can state (C5 probe).
        assert_eq!(emission(Language::Ts, FACT_MODULE_TIER), Some(GRANULAR));
    }

    #[test]
    fn test_tier_fact_distinguishes_the_drivers() {
        assert_eq!(emission(Language::Rust, FACT_TEST_TIER), Some(GRANULAR));
        assert_eq!(emission(Language::Csharp, FACT_TEST_TIER), Some(GRANULAR));
        assert_eq!(
            emission(Language::Go, FACT_TEST_TIER),
            Some(FILE_TIER_ONLY)
        );
        // The ts arm marks test scaffolding per file once C2's evidence and
        // convention co-occur; a file the predicate does not reach stays
        // production, so the mark is the model's, not a name guess.
        assert_eq!(emission(Language::Ts, FACT_TEST_TIER), Some(GRANULAR));
    }

    #[test]
    fn ts_file_grained_and_public_api_rows_state_the_record() {
        // C6 weld of the ts-driver design record: the file-grained row is
        // granular exactly where the driver serializes file-identified facts.
        // rust serializes them in `file_edges` (D11 precondition built); ts
        // keys its module-edge rows by file identity; go and csharp move only
        // at their own later named changes.
        assert_eq!(
            emission(Language::Ts, FACT_FILE_GRAINED_DEPENDENCY_FACTS),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Rust, FACT_FILE_GRAINED_DEPENDENCY_FACTS),
            Some(GRANULAR),
            "the rust driver serializes file-grained facts in the model"
        );
        for language in [Language::Csharp, Language::Go] {
            assert_eq!(
                emission(language, FACT_FILE_GRAINED_DEPENDENCY_FACTS),
                Some(NOT_EMITTED),
                "the {language:?} flip is a later named change"
            );
        }
        // D9: the ts public facts are real named/default-export facts.
        assert_eq!(
            emission(Language::Ts, FACT_PUBLIC_API_SURFACE),
            Some(GRANULAR)
        );
        // The ts arm declares files as vertices but emits no gating marks —
        // the conditional row keeps feature_boundary honest for the language.
        assert_eq!(
            emission(Language::Ts, FACT_ROOT_MODULE_DECLARATIONS),
            Some(DECLARATIONS_NO_GATING)
        );
    }

    #[test]
    fn posture_rows_are_declared_data_and_weld_to_the_capability_table() {
        // D7/ADR-023 since the named flip: rust and ts → both, error;
        // go and csharp → claims, error; python lands at `both` on day one
        // (its rows are file-identified from the first commit); kotlin joins
        // `both` on day one (granular file-facts row from its first commit).
        // The flip carried the counted
        // portfolio cleanup, the serialized file facts (D11) and the
        // zero-count re-probe — the weld below keeps `both` honest: no
        // posture may promise a tier its driver does not emit.
        let rows = |language: Language| -> Vec<(&'static str, &'static str)> {
            POSTURES
                .iter()
                .filter(move |row| row.0 == language)
                .map(|row| (row.1, row.2))
                .collect()
        };
        for language in [Language::Ts, Language::Rust, Language::Python, Language::Kotlin] {
            assert_eq!(rows(language), vec![(SCOPE_BOTH, "error")]);
        }
        for language in [Language::Csharp, Language::Go] {
            assert_eq!(
                rows(language),
                vec![(SCOPE_CLAIMS, "error")],
                "the {language:?} posture row is the claims default (ADR-023)"
            );
        }
        // The weld: every posture row whose scope is `both` requires the
        // file-grained capability granular for that language — a posture
        // can never promise a tier the driver does not emit.
        for (language, scope, _) in POSTURES {
            if *scope == SCOPE_BOTH {
                assert!(
                    emits_granular(*language, FACT_FILE_GRAINED_DEPENDENCY_FACTS),
                    "a both posture for {language:?} requires the granular file-facts row"
                );
            }
        }
        assert_eq!(POSTURES.len(), 6); // one row per language arm
    }

    #[test]
    fn unknown_pairs_lookup_to_none() {
        assert_eq!(emission(Language::Rust, "no-such-fact"), None);
    }

    #[test]
    fn rules_require_facts_the_table_states() {
        for (rule, fact) in RULES {
            for language in [
                Language::Rust,
                Language::Csharp,
                Language::Go,
                Language::Ts,
            ] {
                assert!(
                    emission(language, fact).is_some(),
                    "rule {rule} requires fact {fact} stated for every language"
                );
            }
        }
    }

    #[test]
    fn symbols_fact_is_granular_for_every_language() {
        for language in [Language::Rust, Language::Csharp, Language::Go] {
            assert_eq!(
                emission(language, FACT_SYMBOLS),
                Some(GRANULAR),
                "the sole extraction path carries crossing symbols on module edges"
            );
        }
    }

    #[test]
    fn public_api_surface_fact_distinguishes_the_drivers() {
        // Live-probed fact sources (csharp public-api plan, US 03): rust
        // enumerates crate-root exports (the pinned root-suite behavior),
        // csharp attributes module public types (proven on the issue repro
        // tree and the 3-segment corporate probe in tests/csharp_public_api),
        // and go populates no public API fact — its allowlist stays the
        // fact-emptiness vacuity the engine already announces.
        assert_eq!(
            emission(Language::Rust, FACT_PUBLIC_API_SURFACE),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Csharp, FACT_PUBLIC_API_SURFACE),
            Some(GRANULAR)
        );
        assert_eq!(
            emission(Language::Go, FACT_PUBLIC_API_SURFACE),
            Some(NOT_EMITTED)
        );
    }
}
