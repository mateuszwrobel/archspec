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

/// The structural facade-role check of `verify` (rule name unchanged; the
/// fact source is the model's roles map).
pub const RULE_FACADE_DEPENDENCY: &str = "facade dependency";
/// The ban-routing (conduit) check of `verify`.
pub const RULE_LAUNDERED_FORBIDDEN_EDGE: &str = "laundered forbidden edge";

/// One row per (language, fact), stating what that language's sole extraction
/// path emits (capability-layer data, no engine introspection; every cell was
/// verified against live driver probes). Ordered by fact, then language.
pub const CAPABILITIES: &[(Language, &str, &str)] = &[
    (Language::Rust, FACT_ROLE_FACADE, GRANULAR),
    (Language::Csharp, FACT_ROLE_FACADE, GRANULAR),
    (Language::Go, FACT_ROLE_FACADE, NOT_EMITTED),
    (Language::Rust, FACT_ROLE_COMPOSITION, GRANULAR),
    (Language::Csharp, FACT_ROLE_COMPOSITION, GRANULAR),
    (Language::Go, FACT_ROLE_COMPOSITION, GRANULAR),
    (Language::Rust, FACT_MODULE_TIER, GRANULAR),
    (Language::Csharp, FACT_MODULE_TIER, GRANULAR),
    (Language::Go, FACT_MODULE_TIER, GRANULAR),
    (Language::Rust, FACT_SYMBOLS, GRANULAR),
    (Language::Csharp, FACT_SYMBOLS, GRANULAR),
    (Language::Go, FACT_SYMBOLS, GRANULAR),
    (Language::Rust, FACT_ROOT_MODULE_DECLARATIONS, GRANULAR),
    (Language::Csharp, FACT_ROOT_MODULE_DECLARATIONS, NOT_EMITTED),
    (Language::Go, FACT_ROOT_MODULE_DECLARATIONS, NOT_EMITTED),
    (Language::Rust, FACT_TEST_TIER, GRANULAR),
    (Language::Csharp, FACT_TEST_TIER, GRANULAR),
    (Language::Go, FACT_TEST_TIER, FILE_TIER_ONLY),
    (Language::Rust, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Csharp, FACT_EXTERNAL_PACKAGES, GRANULAR),
    (Language::Go, FACT_EXTERNAL_PACKAGES, GRANULAR),
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
        for language in [Language::Rust, Language::Csharp, Language::Go] {
            let rows: Vec<(&str, &str)> = CAPABILITIES
                .iter()
                .filter(|(row_language, _, _)| *row_language == language)
                .map(|(_, fact, emission)| (*fact, *emission))
                .collect();
            assert_eq!(rows.len(), 7, "{language:?} must state all seven facts");
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
    }

    #[test]
    fn role_composition_fact_is_granular_for_every_language() {
        // Every driver derives composition from its own facts: rust from the
        // wiring bin main root, csharp from DI-registration entrypoints, go
        // from the package-main module.
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
    }

    #[test]
    fn test_tier_fact_distinguishes_the_drivers() {
        assert_eq!(emission(Language::Rust, FACT_TEST_TIER), Some(GRANULAR));
        assert_eq!(emission(Language::Csharp, FACT_TEST_TIER), Some(GRANULAR));
        assert_eq!(
            emission(Language::Go, FACT_TEST_TIER),
            Some(FILE_TIER_ONLY)
        );
    }

    #[test]
    fn unknown_pairs_lookup_to_none() {
        assert_eq!(emission(Language::Rust, "no-such-fact"), None);
    }

    #[test]
    fn rules_require_facts_the_table_states() {
        for (rule, fact) in RULES {
            for language in [Language::Rust, Language::Csharp, Language::Go] {
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
}
