// Shared spec model for the landed consumers: diagram, verify, update and
// report each read what they need (see `verify/compare/` and `commands/`).

use crate::archspec::language;
use crate::archspec::model::{DeclaredPattern, DeclaredPatternVerb};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

pub const SPEC_FILE: &str = "architecture.spec.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    pub project: Project,
    #[serde(default)]
    pub stereotype: Vec<Stereotype>,
    #[serde(rename = "module", default)]
    pub modules: Vec<Module>,
    #[serde(default)]
    pub constraint: Vec<Constraint>,
    /// The `[[exempt]]` ratchet ledger (GitHub issue #27): one entry per
    /// named ring whose debt is accepted, bounded by tier, optional count
    /// cap and optional expiry. Matched by identity, never by count — see
    /// `verify::compare::cycles` for the adjudication and
    /// `helptext::DIAGNOSTICS_HELP` for the emitted categories.
    #[serde(default)]
    pub exempt: Vec<Exempt>,
    /// The `[[async_depends]]` declaration table (workplan
    /// archspec_async_depends, D1): one entry per asynchronous (event)
    /// dependency edge, carrying the event IDENTITY only — `from`, `to`
    /// and `event`. Payload shapes are derived from source on both sides,
    /// never declared (D2). Validated by the three identity arms of
    /// `verify::compare::async_depends` whenever the table is non-empty;
    /// an empty table leaves every code path byte-identical to the
    /// pre-table binary (D8).
    #[serde(default)]
    pub async_depends: Vec<AsyncDepend>,
    /// The `[[canonical_producer]]` declaration face (ADR-069 clause
    /// 4.4a(iv), workplan events_value_join scenario 02): one entry per
    /// fact class naming its ONE canonical producer — the arbitration
    /// precondition that two eligible producers merge instead of
    /// competing (4.4a's identity rule (iv)). Generic by law: the entry
    /// names an opaque fact class and a tree-root-canonical unit id, no
    /// framework or ecosystem vocabulary. Validated by
    /// `verify::compare::declarations` whenever either declaration table
    /// is non-empty; both tables empty leaves every code path
    /// byte-identical to the pre-face binary (the D8 posture).
    #[serde(default)]
    pub canonical_producer: Vec<CanonicalProducer>,
    /// The `[[shared_constant]]` declaration face (ADR-069 4.4a(iv)'s
    /// shared-constant class, survey witness kind 7 — one constant,
    /// several readers, no edge to name): the canonical-producer
    /// declaration over a constant fact class. NEVER an async row — the
    /// declaration schema carries no emit/subscribe keys, and the walker
    /// refuses a constant entry naming one by name (F-W3's negative law).
    #[serde(default)]
    pub shared_constant: Vec<SharedConstant>,
    /// The `[[pattern_pair]]` declaration table (workplan events_value_join
    /// scenario 03, ADR-069 4.4a(v)): one entry per declared emit/subscribe
    /// pattern pair, each side declaring its matcher verb and driver
    /// language over the event class. The engine carries ONLY the two
    /// matcher verbs — every name is declared data here (F2 drafting law,
    /// `rust-arch-test-kit/AGENTS.md` §Archspec engine boundary). Validated
    /// by the fail-closed walker whenever the table is non-empty; an empty
    /// table leaves every code path byte-identical to the pre-face binary
    /// (the D8 posture, 4.2 arm 1).
    #[serde(default)]
    pub pattern_pair: Vec<PatternPair>,
}

/// One `[[async_depends]]` entry: the event identity of one asynchronous
/// dependency. The keys are mandatory per entry — `validate_unknown_keys`
/// refuses an absent or empty one in the shipped invalid-spec class,
/// naming the key and the entry (the `parse_allowed_entry` message class).
/// Fields are `Option` on purpose: the walker runs BEFORE deserialization
/// at every loader, so the walker — not serde — owns the missing-key voice.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AsyncDepend {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
}

/// One `[[canonical_producer]]` entry: the fact class and its ONE
/// canonical producer. The keys are mandatory per entry —
/// `validate_unknown_keys` refuses an absent or empty one in the shipped
/// invalid-spec class, naming the key and the entry (the
/// `check_async_depend_entry` message shape). `producer` must be a
/// tree-root-canonical unit id: a bare per-driver name is refused by name
/// by `verify::compare::declarations`, never canonicalized into an id (a
/// declaration carries no producer directory to anchor the dot's
/// ownership arm — see `events::unitid`). Fields are `Option` on purpose:
/// the walker runs BEFORE deserialization at every loader, so the walker —
/// not serde — owns the missing-key voice.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CanonicalProducer {
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub producer: Option<String>,
}

/// One `[[shared_constant]]` entry: the shared-constant fact class and the
/// canonical producer owning the constant. Same mandatory-key and
/// canonical-id laws as [`CanonicalProducer`]; additionally the walker
/// refuses an entry naming an `emit` or `subscribe` key by name — the
/// shared-constant class is NEVER expressible as a pattern pair (ADR-069
/// 4.4a(iv), F-W3), so those keys never enter the shape and the schema
/// never advertises them.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SharedConstant {
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub producer: Option<String>,
}

/// One `[[pattern_pair]]` entry (workplan events_value_join scenario 03,
/// ADR-069 4.4a(v), decision-check F2/F3): a declared emit/subscribe
/// pattern pair — the id literal-site rows carry as the
/// declared-pattern-id, and the two pattern sides the funded drivers match
/// their candidates against. The `emit`/`subscribe` keys are exactly the
/// pair [`CONSTANT_PATTERN_KEYS`] refuses on a `[[shared_constant]]` entry:
/// the pair belongs to THIS face and never to a constant class (F-W3's
/// negative law). Fields are `Option` on purpose: the walker runs BEFORE
/// deserialization at every loader, so the walker — not serde — owns the
/// missing-key voice.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PatternPair {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub emit: Option<PatternSide>,
    #[serde(default)]
    pub subscribe: Option<PatternSide>,
}

/// One pattern side of a [`PatternPair`]: the driver language the side
/// applies to, its optional enumerated `role` (the SOLE orientation source
/// — 4.4a(v)/F3: the engine carries no name-inference arm, and an absent
/// role is the unoriented state the value join routes to a named
/// `ambiguous-leg` row), and exactly ONE matcher verb: call-name+argument
/// position (`call` + `arg`), exported-property value (`property`), or
/// field value (`field`, workplan visibility_phase2 scenario 02).
/// Generic by law: every field is data — the nouns live here, the verbs
/// live in `events::patterns` (F2 drafting law).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PatternSide {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub call: Option<String>,
    #[serde(default)]
    pub arg: Option<usize>,
    #[serde(default)]
    pub property: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
}

/// Flatten one `[[pattern_pair]]` entry to its pattern-side facts: entry
/// order, the `emit` side before the `subscribe` side (the declaration's
/// own order). A side whose verb arm the walker refused (it cannot reach
/// this arm through a load that passed, but the flattening stays
/// fail-closed) contributes nothing — flattening never invents a verb.
/// The pair id is the declared-pattern-id the literal-site rows carry.
pub fn pattern_sides(pair: &PatternPair) -> Vec<DeclaredPattern> {
    let mut sides = Vec::new();
    for side in [pair.emit.as_ref(), pair.subscribe.as_ref()].into_iter().flatten() {
        let verb = match (&side.call, side.arg, &side.property, &side.field) {
            (Some(call), Some(arg), None, None) => DeclaredPatternVerb::CallNameArgPosition {
                call: call.clone(),
                arg,
            },
            (None, None, Some(property), None) => DeclaredPatternVerb::ExportedPropertyValue {
                property: property.clone(),
            },
            (None, None, None, Some(field)) => DeclaredPatternVerb::FieldValue {
                field: field.clone(),
            },
            _ => continue,
        };
        sides.push(DeclaredPattern {
            id: pair.id.clone().unwrap_or_default(),
            language: side.language.clone().unwrap_or_default(),
            role: side.role.clone(),
            verb,
        });
    }
    sides
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub language: String,
    /// Declared framework activation (round-8 framework-config doctrine):
    /// a SINGLE optional name from the selected driver's registry
    /// (`language::frameworks`) — never auto-discovered, never a list.
    /// Absent keeps every driver at its framework-less semantics; an
    /// unknown name fails the load (the `validate_schema` arm).
    #[serde(default)]
    pub framework: Option<String>,
    /// Declared source exclusions (ts-driver record, D14): an exclusion that
    /// hides language-bearing files must say why, in the report, every run.
    /// Empty by default — specs that never use the key are unaffected.
    #[serde(default)]
    pub excluded_sources: Vec<ExcludedSource>,
}

/// One `[project] excluded_sources` entry: `path` and `reason` are both
/// mandatory — serde defaults them to `None` so the schema validator can
/// name the entry (1-indexed) and the missing key, the same message class
/// `validate_constraint` prints. `mode` is the issue-8 phase-2 dial: an
/// optional selector over the value domain `EXCLUSION_MODE_VALUES`,
/// defaulted at load to the shipped `abort` semantics. The value domain is
/// refused fail-closed by `validate_unknown_keys`, so no verb can ride an
/// out-of-domain value as a silently-ignored dial (the phase-1 sequencing
/// law carried into the value shape).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExcludedSource {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

/// The value domain of the entry-level `mode` key (GitHub issue 8 phase 2,
/// decision-check BLOCKER-2): `abort` is the shipped behavior — a run whose
/// resolution hits the entry fails loud naming the entry; `warn` drops the
/// hit and emits a warning-severity finding naming entry+target, which
/// fails `--strict` and is green without it — never an abort.
pub const EXCLUSION_MODE_VALUES: [&str; 2] = ["abort", "warn"];

#[derive(Debug, Clone, Deserialize)]
pub struct Stereotype {
    pub name: String,
    #[serde(rename = "match", default)]
    pub match_set: MatchSet,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MatchSet {
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub units: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub name: String,
    #[serde(rename = "matches", default)]
    pub matches: MatchSet,
    #[serde(default)]
    pub contract: Contract,
    #[serde(default)]
    pub allowed: Allowed,
    #[serde(default)]
    pub submodules: Vec<Module>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Contract {
    /// `Option` distinguishes an absent key from a declared set: `None` means
    /// the claim makes no expose claim at all (verdict-neutral — never an
    /// allow-all and never a deny-all gate), `Some(set)` binds the surface
    /// crossing this claim's boundary under the chain law (every declaring
    /// ancestor binds; nesting may narrow, never widen). Empty `Some([])` is
    /// a declared deny-all ceiling for the declaring claim's surface.
    #[serde(default)]
    pub expose: Option<Vec<String>>,
    #[serde(default)]
    pub forbid: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Allowed {
    /// True when the spec carried the `[module.allowed]` table itself. The
    /// jurisdiction resolver reads this marker: a nested claim WITHOUT a
    /// declared table inherits the nearest ancestor that has one (D2 of the
    /// submodule-dependency-engagement design), while a declared-but-empty
    /// table is a deliberate deny-all ceiling.
    pub declared: bool,
    pub depend_on: Vec<AllowedEntry>,
    pub forbidden: Vec<AllowedEntry>,
}

/// One entry of `allowed.depend_on`/`allowed.forbidden`. The shipped shape
/// is a plain target string, which takes the `count` default; the table
/// shape `{ target, type_only }` carries the entry-level exemption key
/// (D3 of the ts-driver design record). The value domain is validated at
/// spec load and the dependency adjudication reads the declaration: `count`
/// counts every crossing like any other edge — byte-identical to the
/// shipped law — while `exempt` admits the pair's type-only crossing and
/// still reddens the value crossing. On models without the kind fact
/// (rust, go, csharp) every crossing is a value crossing. The cycle tier
/// knows no exemption: it counts every edge regardless of kind.
#[derive(Debug, Clone)]
pub struct AllowedEntry {
    pub target: String,
    /// The per-entry dependency law selector, stored at load so adjudication
    /// reads what the spec declared instead of re-parsing the file
    /// (`TYPE_ONLY_VALUES`, D3).
    pub type_only: &'static str,
}

/// The value domain of the entry-level `type_only` key.
pub const TYPE_ONLY_VALUES: [&str; 2] = ["count", "exempt"];

impl AllowedEntry {
    /// The plain-string shape: the `count` default.
    fn plain(target: String) -> Self {
        Self {
            target,
            type_only: "count",
        }
    }
}

/// Validate one `depend_on`/`forbidden` array element (1-indexed `ordinal`
/// in the error message) into an entry. Plain strings take the `count`
/// default; tables carry a mandatory string `target` and an optional
/// `type_only` from the named value domain.
fn parse_allowed_entry(list: &str, ordinal: usize, value: toml::Value) -> Result<AllowedEntry, String> {
    match value {
        toml::Value::String(target) => Ok(AllowedEntry::plain(target)),
        toml::Value::Table(table) => {
            let mut keys = table.keys().collect::<Vec<_>>();
            keys.retain(|key| key.as_str() != "target" && key.as_str() != "type_only");
            if !keys.is_empty() {
                keys.sort();
                return Err(format!(
                    "{list} entry #{ordinal}: unknown key(s) {} — supported keys are \"target\" and \"type_only\"",
                    keys.iter()
                        .map(|key| format!("\"{key}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            let Some(target) = table.get("target").and_then(toml::Value::as_str) else {
                return Err(format!(
                    "{list} entry #{ordinal}: missing mandatory key \"target\" — a table entry \
                     states the target and an optional \"type_only\""
                ));
            };
            let type_only = match table.get("type_only") {
                None => "count",
                Some(toml::Value::String(value)) if TYPE_ONLY_VALUES.contains(&value.as_str()) => {
                    if value == "exempt" {
                        "exempt"
                    } else {
                        "count"
                    }
                }
                Some(other) => {
                    return Err(format!(
                        "{list} entry #{ordinal}: type_only value {other} is not supported — \
                         supported values are \"count\" and \"exempt\""
                    ));
                }
            };
            Ok(AllowedEntry {
                target: target.to_string(),
                type_only,
            })
        }
        other => Err(format!(
            "{list} entry #{ordinal}: expected a target string or a {{ target, type_only }} \
             table, found {other}"
        )),
    }
}

/// Deserialize with the `declared` marker: a present table is declared even
/// when both lists are empty; an absent key is not. Entry elements are
/// validated per `parse_allowed_entry`.
impl<'de> serde::Deserialize<'de> for Allowed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Fields {
            #[serde(default)]
            depend_on: Vec<toml::Value>,
            #[serde(default)]
            forbidden: Vec<toml::Value>,
        }
        let fields = Fields::deserialize(deserializer)?;
        let mut depend_on = Vec::with_capacity(fields.depend_on.len());
        for (index, value) in fields.depend_on.into_iter().enumerate() {
            depend_on.push(parse_allowed_entry("allowed.depend_on", index + 1, value).map_err(serde::de::Error::custom)?);
        }
        let mut forbidden = Vec::with_capacity(fields.forbidden.len());
        for (index, value) in fields.forbidden.into_iter().enumerate() {
            forbidden.push(parse_allowed_entry("allowed.forbidden", index + 1, value).map_err(serde::de::Error::custom)?);
        }
        Ok(Self {
            declared: true,
            depend_on,
            forbidden,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Constraint {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub severity: String,
    /// `no_cycles` adjudication scope (ts-driver design record D5):
    /// `"claims"` | `"both"`. Optional — an absent value derives from the
    /// per-language posture table. Validation rejects the field on every
    /// other row kind and rejects any value outside the two above.
    #[serde(default)]
    pub scope: Option<String>,
    /// The `no_cycles` edge-kind selector (the `no_cycles` arm of `verify`,
    /// adjudicated in `verify/compare/cycles.rs`; GitHub issue #22): the
    /// ring classes this constraint adjudicates — `"runtime"` (every leg
    /// carries a value edge) and `"type-assisted"` (some leg is type-only).
    /// Absent or empty = every kind: the shipped behavior, backward
    /// compatible. Validation rejects the field on every other row kind and
    /// rejects any value outside the two above.
    #[serde(default)]
    pub kinds: Vec<String>,
    // `public_api_allowlist`
    #[serde(default)]
    pub allowed: Vec<String>,
    // `forbid_external_crates` and `forbid_submodule_dependency`
    #[serde(default)]
    pub from: Vec<String>,
    #[serde(default)]
    pub forbid: Vec<String>,
    // `manifest_integrity`
    #[serde(default)]
    pub require_publish: Option<bool>,
    #[serde(default)]
    pub forbidden_dependencies: Vec<String>,
    #[serde(default)]
    pub required_features: Vec<String>,
    // `feature_boundary`
    #[serde(default)]
    pub feature: String,
    #[serde(default)]
    pub gated_modules: Vec<String>,
    #[serde(default)]
    pub allowed_from: Vec<String>,
    // `forbid_submodule_dependency`
    #[serde(default)]
    pub parent: String,
}

/// One `[[exempt]]` ratchet entry (GitHub issue #27): the named ring whose
/// debt is accepted, the tier it was seen on, an optional shrink-only match
/// cap and an optional expiry date. `cycle` and `tier` are mandatory — kept
/// as `Option` so the schema validator names the missing key per entry the
/// same way `validate_constraint` words the constraint rows;
/// `validate_exempt` enforces the domain before any adjudication reads them.
#[derive(Debug, Clone, Deserialize)]
pub struct Exempt {
    /// Ring identity: `<member> -> <member> -> ... -> <member>` with a
    /// repeated first member — the same bytes a finding states, tier prefix
    /// and kind label excluded.
    #[serde(default)]
    pub cycle: Option<String>,
    /// Which tier's findings this entry suppresses: `"file"`, `"module"`
    /// or `"both"`. Tier mismatch never suppresses (R2): a file-tier entry
    /// never whitens a module-tier ring, and the reverse.
    #[serde(default)]
    pub tier: Option<String>,
    /// Shrink-only per-entry cap on the findings one identity can suppress
    /// (R1: identity matches, count bounds). Absent = no bound. `0` is
    /// refused — an entry that exempts nothing is a stale entry, not a cap.
    #[serde(default)]
    pub count: Option<i64>,
    /// Optional expiry `YYYY-MM-DD` (UTC). A past date exempts NOTHING (R4):
    /// the debt reports and the entry warns `exempt expired` instead of
    /// whitening or silently vanishing.
    #[serde(default)]
    pub until: Option<String>,
    /// Free-text justification, documentation only — never adjudicated.
    #[serde(default)]
    #[allow(dead_code)] // contract-visible: the [[exempt]] schema declares it; nothing adjudicates by design
    pub reason: Option<String>,
}

impl Exempt {
    /// True when the entry's `until` date lies before `today` (both
    /// `YYYY-MM-DD`, so the lexicographic comparison is the date
    /// comparison). An absent expiry never expires.
    pub fn expired(&self, today: &str) -> bool {
        self.until
            .as_deref()
            .is_some_and(|until| until < today)
    }

    /// True when this entry suppresses a finding on `tier` — the entry's
    /// declared tier, or either tier for `"both"` (R2).
    pub fn matches_tier(&self, tier: &str) -> bool {
        matches!(self.tier.as_deref(), Some(t) if t == "both" || t == tier)
    }
}

/// Today's UTC date as `YYYY-MM-DD` — the clock the expiry comparison runs
/// on (no dependency: civil-from-days over the unix epoch).
pub fn today_iso() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 to the
/// proleptic Gregorian (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 5 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// Every declared claim name at every depth, depth-first. Claim names are
/// globally unique (the schema validator enforces it), so this list is also
/// the set of valid `allowed.depend_on`/`allowed.forbidden` targets (D6).
pub fn claim_names(modules: &[Module]) -> Vec<String> {
    let mut names = Vec::new();
    for module in modules {
        names.push(module.name.clone());
        names.extend(claim_names(&module.submodules));
    }
    names
}

pub fn load(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(crate::archspec::cli::operational_error(format!(
            "no spec found under: {}",
            dir.display()
        )));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("failed to read spec: {}: {err}", spec_path.display()))?;
    validate_unknown_keys(&content, &spec_path)?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|err| format!("failed to parse spec: {}: {err}", spec_path.display()))?;
    for module in &spec.modules {
        validate_submodule_units(&spec_path, module)?;
    }
    Ok(spec)
}

/// Verify's spec loader: same file and parsing, but verify's contract words
/// the errors differently from diagram's (`load` above). Missing spec names the
/// expected file and location; TOML/schema failures are `invalid spec: <file>`.
/// Schema validation is applied after parsing so a malformed but parseable
/// spec is rejected before extraction; the unknown-key refusal
/// (`validate_unknown_keys`) runs before deserialization in every loader, so
/// an undefined key is refused by all verbs that read a spec.
pub fn load_verify(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(crate::archspec::cli::operational_error(format!(
            "no architecture.spec.toml found in: {}",
            dir.display()
        )));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_unknown_keys(&content, &spec_path)?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_schema(&spec_path, &spec)?;
    Ok(spec)
}

/// Report's spec loader: same file and parsing, but report's contract words the
/// errors differently (commands/report/errors.md). A missing spec names the
/// expected file path; a malformed spec is `invalid spec: <file> (malformed
/// TOML)`. Schema validation is applied after parsing, matching load_verify.
pub fn load_report(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(crate::archspec::cli::operational_error(format!(
            "spec file not found: {}",
            spec_path.display()
        )));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_unknown_keys(&content, &spec_path)?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|_| format!("invalid spec: {} (malformed TOML)", spec_path.display()))?;
    validate_schema(&spec_path, &spec)?;
    Ok(spec)
}

/// Scan's root-only spec discovery (ts-driver record, D14(4)): the spec file
/// is read ONLY when it sits at the scanned root — never an upward walk,
/// never a config indirection. A missing spec is not an error for `scan`
/// (the verb never requires one); a present spec is validated exactly like
/// `load_verify` validates, so an invalid spec fails `scan` with the shipped
/// invalid-spec class — untagged in `cli::classify`, keeping exit 1.
pub fn load_scan(dir: &Path) -> Result<Option<Spec>, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_unknown_keys(&content, &spec_path)?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_schema(&spec_path, &spec)?;
    Ok(Some(spec))
}

/// The verb-site pin (GitHub issue #38): a present spec's `[project] language`
/// SELECTS the driver at every detect()-consuming verb — the same shared
/// parser the scan and verify sites call (round-8 F11 lineage), so an unknown
/// name fails with the shipped unknown-language red, byte-identical to the
/// verify-site mapping, BEFORE any source scanning.
pub fn declared_driver(loaded: &Spec) -> Result<language::Language, String> {
    language::from_name(&loaded.project.language).ok_or_else(|| {
        format!(
            "invalid spec: unknown language \"{}\"",
            loaded.project.language
        )
    })
}

/// The `excluded sources` report line (D13 owner: the scan/verify report
/// surface): every active entry, one line per entry, exact bytes
/// `excluded sources: <path> — <reason>` — a run with an active exclusion
/// that prints no line is red-by-test (R3-7). Printed on stderr so the
/// stdout machine body stays pure (wave-3 D02) and rust stdout verdict
/// bytes stay byte-identical.
pub fn excluded_sources_report(spec: &Spec) -> String {
    let mut out = String::new();
    for entry in &spec.project.excluded_sources {
        // The schema validator admits only entries with both keys present
        // and non-empty; the fallbacks keep this line total for callers
        // that skipped validation.
        let path = entry.path.as_deref().unwrap_or_default();
        let reason = entry.reason.as_deref().unwrap_or_default();
        out.push_str(&format!("excluded sources: {path} — {reason}\n"));
    }
    out
}

/// The declared exclusions as entries (path + mode) for the consumers that
/// adjudicate per hit: the ts driver's exclusion naming and mode lookup.
/// Precedence is by specificity, never spec order (decision-check MAJOR-1)
/// — see [`matching_exclusion`]. The schema validator has admitted only
/// non-empty paths by the time this runs; the value-domain validator has
/// admitted only the two `EXCLUSION_MODE_VALUES`, so the mode here is
/// total: an absent key takes the shipped `abort` default.
pub fn exclusion_entries(spec: &Spec) -> Vec<ExclusionEntry> {
    spec.project
        .excluded_sources
        .iter()
        .filter_map(|entry| {
            let path = entry.path.as_deref()?.trim();
            (!path.is_empty()).then(|| ExclusionEntry {
                path: std::path::PathBuf::from(path),
                mode: match entry.mode.as_deref() {
                    Some("warn") => "warn",
                    // `validate_unknown_keys` refuses every other value; the
                    // absent key (and any caller that skipped validation)
                    // keeps the shipped semantics.
                    _ => "abort",
                },
            })
        })
        .collect()
}

/// One consumed exclusion entry: the trimmed root plus the resolved mode.
/// The entry vocabulary and its specificity resolution live in the library's
/// model (the scan drivers adjudicate through them; the loader builds them
/// here), re-exported so the spec surface keeps naming them in place.
pub use rust_arch_test_kit::model::{ExclusionEntry, exclusion_specificity};

/// The zero-match liveness report (decision-check MAJOR-2): one line per
/// entry that matched NOTHING the run would have analyzed, exact bytes
/// `excluded source matched nothing: <path> — <reason>`, printed on the
/// report surface right after the active-entry lines, once per entry per
/// run. An entry counts as matched when at least one file sits at or under
/// its path, behind the mechanical exclusion set (a subtree the walk never
/// visits cannot keep a dead entry alive). The walk is driver-independent:
/// any file counts, so a source-extension narrowing can never declare a
/// populated entry dead.
pub fn exclusion_liveness_report(root: &std::path::Path, spec: &Spec) -> String {
    // Liveness reads paths and reasons; the mode plays no part. The pairing
    // mirrors `exclusion_entries` (trim, non-empty) so the two agree.
    let entries: Vec<(std::path::PathBuf, &str)> = spec
        .project
        .excluded_sources
        .iter()
        .filter_map(|entry| {
            let path = entry.path.as_deref()?.trim();
            (!path.is_empty()).then(|| {
                (
                    std::path::PathBuf::from(path),
                    entry.reason.as_deref().unwrap_or_default(),
                )
            })
        })
        .collect();
    if entries.is_empty() {
        return String::new();
    }
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut matched = vec![false; entries.len()];
    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_entry(|entry| {
            entry.path() == root
                || !entry
                    .path()
                    .strip_prefix(&root)
                    .map(|rel| {
                        rel.components().any(|component| {
                            matches!(component, std::path::Component::Normal(name)
                                if crate::archspec::language::is_excluded_dir(&name.to_string_lossy()))
                        })
                    })
                    .unwrap_or(false)
        })
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(&root) else {
            continue;
        };
        for (index, (declared, _)) in entries.iter().enumerate() {
            if !matched[index] && exclusion_specificity(declared, rel).is_some() {
                matched[index] = true;
            }
        }
    }
    let mut out = String::new();
    for (index, (path, reason)) in entries.iter().enumerate() {
        if !matched[index] {
            out.push_str(&format!(
                "excluded source matched nothing: {} — {reason}\n",
                path.display()
            ));
        }
    }
    out
}

/// The declared exclusion roots as paths, for the counted walks (D14: the
/// agreement check consumes the entries the report line prints). The schema
/// validator has admitted only non-empty paths by the time this runs;
/// entries without a path contribute nothing. Consumers that adjudicate a
/// per-hit mode read [`exclusion_entries`] instead.
pub fn exclusion_paths(spec: &Spec) -> Vec<std::path::PathBuf> {
    exclusion_entries(spec)
        .into_iter()
        .map(|entry| entry.path)
        .collect()
}

/// The known keys of every spec shape (GitHub issue 8 phase 1, decision-check
/// BLOCKER-3): the SINGLE source of truth consumed by both the runtime
/// unknown-key validator (`validate_unknown_keys`) and the JSON Schema
/// generator (`commands::spec`), so the two cannot drift — the same role
/// `CONSTRAINT_REQUIRED_FIELDS` plays for per-type required fields. Each
/// shape's keys are sorted: refusal messages and schema properties stay
/// byte-stable. The boundary is the whole surface the model deserializes:
/// the top-level tables, `[project]` and its `excluded_sources` entries,
/// stereotype entries and their `match` tables, module claims (submodules
/// recursively) and their `matches`/`contract`/`allowed` tables, and
/// constraint entries. The `{ target, type_only }` entry tables under
/// `allowed` are refused by `parse_allowed_entry`, already, from inside
/// deserialization.
pub const SPEC_KEY_SHAPES: &[(&str, &[&str])] = &[
    ("allowed", &["depend_on", "forbidden"]),
    ("async_depends", &["event", "from", "to"]),
    ("canonical_producer", &["class", "producer"]),
    (
        "constraint",
        &[
            "allowed",
            "allowed_from",
            "feature",
            "forbid",
            "forbidden_dependencies",
            "from",
            "gated_modules",
            "kinds",
            "modules",
            "parent",
            "require_publish",
            "required_features",
            "scope",
            "severity",
            "type",
        ],
    ),
    ("contract", &["expose", "forbid"]),
    ("excluded_sources", &["mode", "path", "reason"]),
    ("exempt", &["count", "cycle", "reason", "tier", "until"]),
    ("match", &["modules", "names", "paths", "units"]),
    ("module", &["allowed", "contract", "matches", "name", "submodules"]),
    ("pattern_pair", &["emit", "id", "subscribe"]),
    ("pattern_side", &["arg", "call", "field", "language", "property", "role"]),
    ("project", &["excluded_sources", "framework", "language"]),
    ("shared_constant", &["class", "producer"]),
    ("stereotype", &["match", "name"]),
    (
        "top",
        &[
            "async_depends",
            "canonical_producer",
            "constraint",
            "exempt",
            "module",
            "pattern_pair",
            "project",
            "shared_constant",
            "stereotype",
        ],
    ),
];

/// The pattern-pair keys a `[[shared_constant]]` entry may never name
/// (ADR-069 4.4a(iv), F-W3): the join-pattern pair belongs to the
/// pattern-pair declaration face, NEVER to a constant class — a constant
/// has no edge to name, so an emit/subscribe key on a constant entry is
/// the named load-time refusal (`check_constant_pattern_pair`), not a
/// silently-ignored key and not the generic unknown-key voice. The pair is
/// ONE constant read by the walker and advertised-nowhere by the schema
/// generator, so reference and validator cannot drift on it either.
pub const CONSTANT_PATTERN_KEYS: [&str; 2] = ["emit", "subscribe"];

/// The value domain of a `[[pattern_pair]]` side's `role` key (workplan
/// events_value_join scenario 03, ADR-069 4.4a(v), F3): orientation comes
/// ONLY from this enumerated role — the engine carries no name-inference
/// arm, so `publish`/`subscribe` are the only words that orient a matched
/// site. An absent role is legal and states "unoriented" (the value join
/// routes it to the named `ambiguous-leg` row, 4.4a(v)); an out-of-domain
/// role value is the shipped `invalid spec` class (`check_pattern_role`,
/// the `check_exclusion_mode` posture — fail closed, never a silently
/// ignored dial). The pair is ONE constant read by the walker and
/// advertised by the schema generator, so reference and validator cannot
/// drift on it.
pub const PATTERN_ROLE_VALUES: [&str; 2] = ["publish", "subscribe"];

/// The declared keys of one shape. Panics on an unknown shape name — the
/// walker and the schema generator address only the shapes above.
pub fn shape_keys(shape: &str) -> &'static [&'static str] {
    SPEC_KEY_SHAPES
        .iter()
        .find(|(name, _)| *name == shape)
        .unwrap_or_else(|| panic!("undeclared spec key shape {shape}"))
        .1
}

/// Fail-closed unknown-key refusal (GitHub issue 8 phase 1, BLOCKER-3): a key
/// the model does not define is an `invalid spec` error naming the unexpected
/// key(s) and where the key sits. Every spec loader calls this before
/// deserialization, so `verify`, `report`, `scan` and `diagram` all refuse a
/// stale-key spec — the loudness is the point: a typo'd key can never ride a
/// silently-ignored future semantic (the sequencing law of the decision-check).
/// Phase 2 extends the same fail-closed law to the dial's VALUES: an
/// `excluded_sources` entry whose `mode` is outside `EXCLUSION_MODE_VALUES`
/// is refused in the same class (`check_exclusion_mode`). Malformed TOML
/// stays silent here — each loader's own parse-error class owns
/// a file that does not parse. Reporting order is deterministic: the top
/// level, then `[project]` (its keys, then entries in order), then stereotypes,
/// then module claims depth-first, then constraints.
fn validate_unknown_keys(content: &str, spec_path: &std::path::Path) -> Result<(), String> {
    let value = match content.parse::<toml::Value>() {
        Ok(value) => value,
        // A parse failure stays silent here — each loader's own malformed-TOML
        // class owns files that do not parse — EXCEPT the module `allowed`
        // duplicate-declaration class (GitHub issue #58), named below instead
        // of letting the raw duplicate-key error context-bleed.
        Err(err) => {
            return duplicate_allowed_parse_error(content, &err, spec_path).map_or(Ok(()), Err)
        }
    };
    let Some(root) = value.as_table() else {
        return Ok(());
    };
    check_spec_keys(root, "top", "at the top level", spec_path)?;
    if let Some(project) = root.get("project").and_then(toml::Value::as_table) {
        check_spec_keys(project, "project", "in [project]", spec_path)?;
        for (index, entry) in array_items(project.get("excluded_sources")).iter().enumerate() {
            if let Some(table) = entry.as_table() {
                let location = format!("in [project] excluded_sources entry #{}", index + 1);
                check_spec_keys(table, "excluded_sources", &location, spec_path)?;
                check_exclusion_mode(table, index, spec_path)?;
            }
        }
    }
    for (index, entry) in array_items(root.get("stereotype")).iter().enumerate() {
        let location = format!("in [stereotype] #{}", index + 1);
        let Some(table) = entry.as_table() else { continue };
        check_spec_keys(table, "stereotype", &location, spec_path)?;
        if let Some(matched) = table.get("match").and_then(toml::Value::as_table) {
            check_spec_keys(matched, "match", &format!("{location} match"), spec_path)?;
        }
    }
    validate_module_keys(array_items(root.get("module")), "in [module]", spec_path)?;
    for (index, entry) in array_items(root.get("constraint")).iter().enumerate() {
        let location = format!("in [constraint] #{}", index + 1);
        if let Some(table) = entry.as_table() {
            check_spec_keys(table, "constraint", &location, spec_path)?;
        }
    }
    for (index, entry) in array_items(root.get("exempt")).iter().enumerate() {
        let location = format!("in [exempt] #{}", index + 1);
        if let Some(table) = entry.as_table() {
            check_spec_keys(table, "exempt", &location, spec_path)?;
        }
    }
    for (index, entry) in array_items(root.get("async_depends")).iter().enumerate() {
        let location = format!("in [async_depends] #{}", index + 1);
        if let Some(table) = entry.as_table() {
            check_spec_keys(table, "async_depends", &location, spec_path)?;
            check_async_depend_entry(table, index, spec_path)?;
        }
    }
    check_async_depend_duplicates(array_items(root.get("async_depends")), spec_path)?;
    // The declaration face (workplan events_value_join scenario 02): the
    // SAME fail-closed walker covers both tables, so `verify`, `report`,
    // `scan` and `diagram` all refuse a malformed declaration in the same
    // invalid-spec class. The constant arm runs BEFORE the generic
    // unknown-key voice so an emit/subscribe key on a constant entry
    // reaches its own named refusal, never the unknown-key echo (F-W3's
    // negative law: the schema never makes a shared constant expressible
    // as a pattern pair).
    for (index, entry) in array_items(root.get("canonical_producer")).iter().enumerate() {
        let location = format!("in [canonical_producer] #{}", index + 1);
        if let Some(table) = entry.as_table() {
            check_spec_keys(table, "canonical_producer", &location, spec_path)?;
            check_declaration_entry(table, "canonical_producer", index, spec_path)?;
        }
    }
    for (index, entry) in array_items(root.get("shared_constant")).iter().enumerate() {
        let location = format!("in [shared_constant] #{}", index + 1);
        if let Some(table) = entry.as_table() {
            check_constant_pattern_pair(table, index, spec_path)?;
            check_spec_keys(table, "shared_constant", &location, spec_path)?;
            check_declaration_entry(table, "shared_constant", index, spec_path)?;
        }
    }
    // The pattern-pair declaration face (workplan events_value_join
    // scenario 03): the SAME fail-closed walker covers the pair id, each
    // side's keys, the ONE-verb law and the enumerated-role value domain,
    // so every loader refuses a malformed pattern before deserialization.
    for (index, entry) in array_items(root.get("pattern_pair")).iter().enumerate() {
        let location = format!("in [pattern_pair] #{}", index + 1);
        let Some(table) = entry.as_table() else { continue };
        check_spec_keys(table, "pattern_pair", &location, spec_path)?;
        check_pattern_pair_entry(table, index, spec_path)?;
        for key in CONSTANT_PATTERN_KEYS {
            if let Some(side) = table.get(key).and_then(toml::Value::as_table) {
                let side_location = format!("{location} {key}");
                check_spec_keys(side, "pattern_side", &side_location, spec_path)?;
                check_pattern_side(side, "pattern_pair", index, key, spec_path)?;
            }
        }
    }
    check_pattern_pair_duplicates(array_items(root.get("pattern_pair")), spec_path)?;
    Ok(())
}

/// The table entries of an optional array value; a non-array (or absent)
/// value contributes nothing — deserialization owns that type error.
fn array_items(value: Option<&toml::Value>) -> &[toml::Value] {
    value
        .and_then(toml::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The module-claim `allowed` duplicate-declaration class (GitHub issue #58):
/// one module table declaring `allowed` BOTH as an inline key and as a
/// `[module.allowed]` header section is malformed TOML — the parser refuses
/// the second declaration, and the raw duplicate-key error context-bleeds
/// (its wording varies with which declaration lands later, and the nesting
/// it implies names the wrong table). Name the collision instead, in the
/// shipped `invalid spec` class, with a deterministic entry ordinal: the
/// module (and submodule, for the `module.submodules` path) ordinals are the
/// array-of-table headers strictly before the parser's reported line — the
/// element the parser's second declaration lands in. Any other parse failure
/// — including an unrelated duplicate-key error — stays silent here, in its
/// loader's parse-error class, byte-for-byte as before.
fn duplicate_allowed_parse_error(
    content: &str,
    err: &toml::de::Error,
    spec_path: &std::path::Path,
) -> Option<String> {
    let message = err.to_string();
    let table = message
        .split("duplicate key `allowed` in table `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())?;
    let line = parse_error_line(&message)?;
    let (modules, submodules) = module_ordinals_at(content, line);
    let location = match table {
        "module" if modules > 0 => format!("module #{modules}"),
        "module.submodules" if modules > 0 && submodules > 0 => {
            format!("module #{modules} submodule #{submodules}")
        }
        _ => return None,
    };
    Some(format!(
        "invalid spec: {}: {location} declares \"allowed\" twice (inline key + [module.allowed] section) — keep one",
        spec_path.display()
    ))
}

/// The 1-indexed line a `toml::de::Error` Display names: the
/// `TOML parse error at line N, column M` banner every parse failure
/// carries. `None` when the banner is absent (an error class the attribution
/// law cannot place on a line) — the caller then leaves the raw error alone.
fn parse_error_line(message: &str) -> Option<usize> {
    let after = message.split_once(" at line ")?.1;
    after
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<usize>()
        .ok()
}

/// The module and submodule ordinals in force at the given 1-indexed line:
/// count the array-of-table headers strictly BEFORE it (the element open at
/// that line), the submodule counter reset by each module header. Header
/// bytes are read comment-stripped and whitespace-stripped; anything that is
/// not a module-header line counts nothing.
fn module_ordinals_at(content: &str, line: usize) -> (usize, usize) {
    let mut modules = 0usize;
    let mut submodules = 0usize;
    for text in content.lines().take(line.saturating_sub(1)) {
        let header: String = text
            .split('#')
            .next()
            .unwrap_or_default()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        match header.as_str() {
            "[[module]]" | "[module]" => {
                modules += 1;
                submodules = 0;
            }
            "[[module.submodules]]" | "[module.submodules]" => submodules += 1,
            _ => {}
        }
    }
    (modules, submodules)
}

/// One table against one shape: any key outside the shape's declared keys is
/// refused, naming the keys (sorted) and the location, with the supported
/// keys listed — the message class `parse_allowed_entry` ships.
fn check_spec_keys(
    table: &toml::map::Map<String, toml::Value>,
    shape: &str,
    location: &str,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    let keys = shape_keys(shape);
    let mut unknown: Vec<&String> = table
        .keys()
        .filter(|key| !keys.contains(&key.as_str()))
        .collect();
    if unknown.is_empty() {
        return Ok(());
    }
    unknown.sort();
    Err(format!(
        "invalid spec: {}: unknown key(s) {} {location} — supported keys are {}",
        spec_path.display(),
        unknown
            .iter()
            .map(|key| format!("\"{key}\""))
            .collect::<Vec<_>>()
            .join(", "),
        keys.iter()
            .map(|key| format!("\"{key}\""))
            .collect::<Vec<_>>()
            .join(", "),
    ))
}

/// The `mode` value domain of one `excluded_sources` entry (issue-8 phase 2,
/// BLOCKER-2): a value outside [`EXCLUSION_MODE_VALUES`] is the shipped
/// invalid-spec class naming the entry, the value and the supported values —
/// the message class `parse_allowed_entry` ships for `type_only`. The check
/// rides the walker (not deserialization) on purpose: every loader calls the
/// walker first, so the fail-closed value law covers `diagram`'s `load` in
/// the same message class as the other three verbs — an out-of-domain dial
/// value can never ride any verb silently, the phase-1 sequencing law
/// extended from keys to values.
fn check_exclusion_mode(
    table: &toml::map::Map<String, toml::Value>,
    index: usize,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    let Some(mode) = table.get("mode") else {
        return Ok(());
    };
    if matches!(mode.as_str(), Some(value) if EXCLUSION_MODE_VALUES.contains(&value)) {
        return Ok(());
    }
    Err(format!(
        "invalid spec: {}: [project] excluded_sources entry #{}: mode value {mode} is not supported — \
         supported values are \"abort\" and \"warn\"",
        spec_path.display(),
        index + 1,
    ))
}

/// The mandatory-key law of one `[[async_depends]]` entry (workplan
/// archspec_async_depends, D1): `from`, `to` and `event` are each required
/// and non-empty — an entry without one is the shipped `invalid spec` class
/// naming the entry (1-indexed) and the key, the `validate_exempt` message
/// shape. The check rides the walker (like `check_exclusion_mode`) so every
/// loader refuses before deserialization. A fourth key is refused by
/// `check_spec_keys` against the `async_depends` shape; nothing here
/// duplicates that voice.
fn check_async_depend_entry(
    table: &toml::map::Map<String, toml::Value>,
    index: usize,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    for key in ["from", "to", "event"] {
        match table.get(key).and_then(toml::Value::as_str) {
            None | Some("") => {
                return Err(format!(
                    "invalid spec: {}: [async_depends] entry #{}: missing mandatory key \
                     \"{key}\" — an entry declares the event identity with from, to and event",
                    spec_path.display(),
                    index + 1,
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// The mandatory-key law of both declaration tables (workplan
/// events_value_join scenario 02): every `[[canonical_producer]]` and
/// `[[shared_constant]]` entry names its fact class and its ONE canonical
/// producer — both keys mandatory per entry, the `check_async_depend_entry`
/// message shape. The check rides the walker so every loader (verify,
/// report, scan, diagram) refuses before deserialization. An unknown key
/// is refused by `check_spec_keys` against the shape row; nothing here
/// duplicates that voice.
fn check_declaration_entry(
    table: &toml::map::Map<String, toml::Value>,
    shape: &str,
    index: usize,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    for key in ["class", "producer"] {
        match table.get(key).and_then(toml::Value::as_str) {
            None | Some("") => {
                return Err(format!(
                    "invalid spec: {}: [{shape}] entry #{}: missing mandatory key \
                     \"{key}\" — an entry declares the fact class and its one canonical \
                     producer with class and producer",
                    spec_path.display(),
                    index + 1,
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// The F-W3 negative law (ADR-069 clause 4.4a(iv)): a shared-constant
/// entry may NEVER name the join-pattern pair — a constant has no edge to
/// name, so an `emit`/`subscribe` key on a `[[shared_constant]]` entry is
/// refused by name, in its own message class, BEFORE the generic
/// unknown-key voice could fold it into an echo. The refused key list is
/// [`CONSTANT_PATTERN_KEYS`]: the same constant the schema generator reads
/// to advertise the keys NOWHERE, so validator and schema cannot drift on
/// the pair's absence.
fn check_constant_pattern_pair(
    table: &toml::map::Map<String, toml::Value>,
    index: usize,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    for key in CONSTANT_PATTERN_KEYS {
        if table.contains_key(key) {
            return Err(format!(
                "invalid spec: {}: [shared_constant] entry #{}: key \"{key}\" — a shared \
                 constant is never a pattern pair: declare the class with class and producer, \
                 a pattern pair belongs to the pattern-pair declaration face",
                spec_path.display(),
                index + 1,
            ));
        }
    }
    Ok(())
}

/// The mandatory-key law of the `[[pattern_pair]]` table (workplan
/// events_value_join scenario 03): every entry names its id — the
/// declared-pattern-id every matched literal-site row carries — in the
/// `check_declaration_entry` message shape. An unknown key is refused by
/// `check_spec_keys` against the shape row; nothing here duplicates that
/// voice. A pair with NEITHER side is refused too: a pattern pair with no
/// pattern states nothing to match, and the never-silent law forbids a
/// declaration that can never produce a row (the `check_declaration_entry`
/// posture extended to the pair's defining keys).
fn check_pattern_pair_entry(
    table: &toml::map::Map<String, toml::Value>,
    index: usize,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    match table.get("id").and_then(toml::Value::as_str) {
        None | Some("") => {
            return Err(format!(
                "invalid spec: {}: [pattern_pair] entry #{}: missing mandatory key \
                 \"id\" — an entry declares the pattern pair with id and its emit/subscribe \
                 pattern sides",
                spec_path.display(),
                index + 1,
            ));
        }
        Some(_) => {}
    }
    if !CONSTANT_PATTERN_KEYS
        .iter()
        .any(|key| table.contains_key(*key))
    {
        return Err(format!(
            "invalid spec: {}: [pattern_pair] entry #{}: no pattern side — an entry declares \
             at least one of its emit/subscribe pattern sides, an empty pair states nothing \
             to match",
            spec_path.display(),
            index + 1,
        ));
    }
    Ok(())
}

/// The side law of a `[[pattern_pair]]` pattern side (workplan
/// events_value_join scenario 03, decisions A2/A4/F3): the side names its
/// driver `language` from the closed driver vocabulary and exactly ONE
/// matcher verb — `call` with `arg` (call-name + argument position) or
/// `property` (exported-property value). No verb states nothing to match
/// and both verbs state an ambiguous site class: each is refused by name in
/// the shipped `invalid spec` class. A `role` that is present must sit in
/// `PATTERN_ROLE_VALUES` (`check_pattern_role`, the `check_exclusion_mode`
/// value-domain posture); an absent role is the legal unoriented state
/// (4.4a(v): the value join surfaces it as the named `ambiguous-leg` row,
/// never as a joined row).
fn check_pattern_side(
    table: &toml::map::Map<String, toml::Value>,
    shape: &str,
    index: usize,
    side: &str,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    let entry = format!("[{shape}] entry #{}, {side}", index + 1);
    match table.get("language").and_then(toml::Value::as_str) {
        None | Some("") => {
            return Err(format!(
                "invalid spec: {}: {entry}: missing mandatory key \"language\" — a pattern \
                 side states the driver language its matcher verb runs on",
                spec_path.display(),
            ));
        }
        Some(language) => {
            if crate::archspec::language::from_name(language).is_none() {
                return Err(format!(
                    "invalid spec: {}: {entry}: unknown driver language \"{language}\" — \
                     a pattern side names one of the driver languages",
                    spec_path.display(),
                ));
            }
        }
    }
    let call = table.get("call").and_then(toml::Value::as_str);
    let arg = table.get("arg").and_then(toml::Value::as_integer);
    let property = table.get("property").and_then(toml::Value::as_str);
    let field = table.get("field").and_then(toml::Value::as_str);
    let verbs = [call, property, field].iter().filter(|verb| verb.is_some()).count();
    match (call, property, field) {
        (None, None, None) => Err(format!(
            "invalid spec: {}: {entry}: no matcher verb — a pattern side declares exactly \
             one verb: call with arg, property, or field",
            spec_path.display(),
        )),
        _ if verbs > 1 => Err(format!(
            "invalid spec: {}: {entry}: declares more than one matcher verb (call, property, \
             field) — a pattern side declares exactly one verb",
            spec_path.display(),
        )),
        (Some(name), None, None) => {
            if name.is_empty() {
                return Err(format!(
                    "invalid spec: {}: {entry}: key \"call\" is empty — the verb's call name \
                     is the declared noun the callee's simple name is compared to",
                    spec_path.display(),
                ));
            }
            match arg {
                None => Err(format!(
                    "invalid spec: {}: {entry}: call verb without arg — the argument position \
                     is mandatory beside the call name (0-based)",
                    spec_path.display(),
                )),
                Some(position) if position < 0 => Err(format!(
                    "invalid spec: {}: {entry}: key \"arg\" is negative — the argument \
                     position is 0-based",
                    spec_path.display(),
                )),
                Some(_) => Ok(()),
            }
        }
        (None, Some(name), None) => {
            if name.is_empty() {
                return Err(format!(
                    "invalid spec: {}: {entry}: key \"property\" is empty — the verb's \
                     property name is the declared noun the exported property's name is \
                     compared to",
                    spec_path.display(),
                ));
            }
            if table.contains_key("arg") || table.contains_key("call") {
                return Err(format!(
                    "invalid spec: {}: {entry}: property verb carries a call-side key — the \
                     exported-property verb takes property alone",
                    spec_path.display(),
                ));
            }
            Ok(())
        }
        (None, None, Some(name)) => {
            if name.is_empty() {
                return Err(format!(
                    "invalid spec: {}: {entry}: key \"field\" is empty — the verb's field \
                     name is the declared noun the keyed pair's key is compared to",
                    spec_path.display(),
                ));
            }
            if table.contains_key("arg")
                || table.contains_key("call")
                || table.contains_key("property")
            {
                return Err(format!(
                    "invalid spec: {}: {entry}: field verb carries a side key of another verb \
                     — the field-value verb takes field alone",
                    spec_path.display(),
                ));
            }
            Ok(())
        }
        // A non-string verb value is the generic typed-key voice elsewhere;
        // reaching this arm means the verb keys are otherwise well-formed.
        _ => Ok(()),
    }?;
    check_pattern_role(table, &entry, spec_path)
}

/// The enumerated-role value domain of a pattern side (F3/A4): the role
/// word is the ONLY orientation source, so an out-of-domain value is
/// refused fail-closed in the shipped `invalid spec` class, naming the
/// entry, the value and the domain (`check_exclusion_mode` posture). An
/// absent `role` key answers Ok — absence is the legal unoriented state,
/// decided where orientation is consumed (the value join, scenario 04).
fn check_pattern_role(
    table: &toml::map::Map<String, toml::Value>,
    entry: &str,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    let Some(value) = table.get("role") else {
        return Ok(());
    };
    let value = value.as_str().unwrap_or_default();
    if !PATTERN_ROLE_VALUES.contains(&value) {
        return Err(format!(
            "invalid spec: {}: {entry}: role \"{value}\" is not one of {} — orientation \
             comes only from the enumerated role",
            spec_path.display(),
            PATTERN_ROLE_VALUES.join("/"),
        ));
    }
    Ok(())
}

/// The exact-duplicate law of the `[[pattern_pair]]` table (the
/// `check_async_depend_duplicates` shape extended to this face): two
/// entries with one id make the declared-pattern-id of a matched row
/// address two declarations, so orientation would resolve twice — an
/// ambiguity no join arm may settle by map order. Refused at load by name
/// with both entries (1-indexed, the walker's numbering).
fn check_pattern_pair_duplicates(
    entries: &[toml::Value],
    spec_path: &std::path::Path,
) -> Result<(), String> {
    let mut ids: Vec<(&str, usize)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(table) = entry.as_table() else { continue };
        let Some(id) = table.get("id").and_then(toml::Value::as_str) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        if let Some((_, first)) = ids.iter().find(|(known, _)| *known == id) {
            return Err(format!(
                "invalid spec: {}: [pattern_pair] entry #{} repeats id \"{}\" declared by \
                 entry #{} — one id addresses one pattern pair",
                spec_path.display(),
                index + 1,
                id,
                first + 1,
            ));
        }
        ids.push((id, index));
    }
    Ok(())
}

/// The exact-duplicate law of the `[[async_depends]]` table (GitHub issue
/// #56): an entry EQUAL on ALL fields to a previously-parsed entry is the
/// shipped `invalid spec` class naming both entries (1-indexed, the walker's
/// numbering). A verbatim re-declaration is a spec authoring accident — a
/// copy-paste or file-duplication bug — and folding it silently would hide
/// it: the renderers deduplicate rows while the entry-level counting
/// (`explain`, the arm markers) counts raw entries, so a duplicate inflates
/// every census while drawing one arrow. Entries differing on ANY field
/// stay ACCEPTED: the same `from`/`to` pair over distinct events is the
/// legal multi-event class (the issue #48 collision class), never a
/// duplicate. Table equality is field-by-field and key-order independent,
/// so a reordered copy speaks the same refusal; a fourth key never reaches
/// this arm because `check_spec_keys` refuses it first. The check rides the
/// walker AFTER the per-entry arms, like `check_exclusion_mode`, so every
/// The exact-duplicate law of the `[[async_depends]]` table (GitHub issue
/// loader refuses before deserialization and explain/entry counting counts
/// DISTINCT facts by construction.
fn check_async_depend_duplicates(
    entries: &[toml::Value],
    spec_path: &std::path::Path,
) -> Result<(), String> {
    for (index, entry) in entries.iter().enumerate() {
        let Some(table) = entry.as_table() else {
            continue; // a non-table entry is deserialization's type error to name
        };
        for (previous, earlier) in entries[..index].iter().enumerate() {
            if earlier.as_table().is_some_and(|earlier| earlier == table) {
                return Err(format!(
                    "invalid spec: {}: duplicate [[async_depends]] entry #{} duplicates #{} — \
                     the table declares each event edge once",
                    spec_path.display(),
                    index + 1,
                    previous + 1,
                ));
            }
        }
    }
    Ok(())
}

/// Module claims and their nested submodules, depth-first; `location` carries
/// the walk path ("in [module] #1 submodule #2"), so a refusal states exactly
/// where the unexpected key sits.
fn validate_module_keys(
    modules: &[toml::Value],
    location: &str,
    spec_path: &std::path::Path,
) -> Result<(), String> {
    for (index, entry) in modules.iter().enumerate() {
        let location = format!("{location} #{}", index + 1);
        let Some(table) = entry.as_table() else { continue };
        check_spec_keys(table, "module", &location, spec_path)?;
        if let Some(matches) = table.get("matches").and_then(toml::Value::as_table) {
            check_spec_keys(matches, "match", &format!("{location} matches"), spec_path)?;
        }
        if let Some(contract) = table.get("contract").and_then(toml::Value::as_table) {
            check_spec_keys(contract, "contract", &format!("{location} contract"), spec_path)?;
        }
        if let Some(allowed) = table.get("allowed").and_then(toml::Value::as_table) {
            // The parseable face of the issue-#58 collision: a `[module.allowed]`
            // section whose body then carries the inline `allowed = { ... }`
            // line is valid TOML nesting — the inline form lands INSIDE the
            // section it re-declares. Name the duplicate declaration instead
            // of surfacing a context-bleeding unknown-key error for the
            // swallowed key.
            if allowed.contains_key("allowed") {
                return Err(format!(
                    "invalid spec: {}: {} declares \"allowed\" twice (inline key + [module.allowed] section) — keep one",
                    spec_path.display(),
                    location.replacen("in [module]", "module", 1),
                ));
            }
            check_spec_keys(allowed, "allowed", &format!("{location} allowed"), spec_path)?;
        }
        validate_module_keys(
            array_items(table.get("submodules")),
            &format!("{location} submodule"),
            spec_path,
        )?;
    }
    Ok(())
}

/// Per-constraint-type list of required non-empty fields. Single source of
/// truth consumed by both the runtime validator (`validate_constraint`) and the
/// JSON Schema generator (`commands::spec`), so the two cannot drift. The empty
/// arrays (`no_cycles`, `manifest_integrity`) declare that no field is required.
pub const CONSTRAINT_REQUIRED_FIELDS: &[(&str, &[&str])] = &[
    ("no_cycles", &[]),
    ("manifest_integrity", &[]),
    ("public_api_allowlist", &["allowed"]),
    ("forbid_external_crates", &["from", "forbid"]),
    ("forbid_submodule_dependency", &["parent", "from", "forbid"]),
    ("feature_boundary", &["feature", "gated_modules"]),
    ("external_free", &["from"]),
];

/// Whether a single required field is empty for the given constraint. List
/// fields are empty when the vector is empty; scalar fields when trimmed empty.
fn required_field_empty(constraint: &Constraint, field: &str) -> bool {
    match field {
        "allowed" => constraint.allowed.is_empty(),
        "from" => constraint.from.is_empty(),
        "forbid" => constraint.forbid.is_empty(),
        "gated_modules" => constraint.gated_modules.is_empty(),
        "feature" => constraint.feature.trim().is_empty(),
        "parent" => constraint.parent.trim().is_empty(),
        _ => false,
    }
}

/// Validate a single constraint's type and required fields. A genuinely
/// unknown `type` is a schema violation naming the file, the entry (1-indexed),
/// and the offending type (#37). Each known type validates the fields its check
/// needs; a missing required field is reported the same way as an unknown type.
fn validate_constraint(
    spec_path: &std::path::Path,
    constraint: &Constraint,
    index: usize,
) -> Result<(), String> {
    let required = |field: &str| {
        Err(format!(
            "invalid spec: {}: [constraint] #{} of type \"{}\" requires a non-empty `{field}` field",
            spec_path.display(),
            index + 1,
            constraint.kind
        ))
    };
    if !CONSTRAINT_REQUIRED_FIELDS
        .iter()
        .any(|(kind, _)| *kind == constraint.kind)
    {
        return Err(format!(
            "invalid spec: {}: unknown constraint type \"{}\" in [constraint] #{}",
            spec_path.display(),
            constraint.kind,
            index + 1
        ));
    }
    for (kind, fields) in CONSTRAINT_REQUIRED_FIELDS {
        if *kind == constraint.kind {
            for field in *fields {
                if required_field_empty(constraint, field) {
                    return required(field);
                }
            }
            break;
        }
    }
    if !constraint.severity.is_empty()
        && constraint.severity != "error"
        && constraint.severity != "warning"
    {
        return Err(format!(
            "invalid spec: {}: constraint #{} has invalid severity \"{}\" (supported: error, warning)",
            spec_path.display(),
            index + 1,
            constraint.severity
        ));
    }
    // The `scope` field of the no_cycles arm (ts-driver design record D5):
    // accepted only on `no_cycles` rows; the two supported values are named
    // in the invalid-value message, same shipped invalid-spec class as the
    // severity check above. The round-1 `files` value is deleted (B1) — it
    // lands in the same invalid-value class.
    if let Some(scope) = &constraint.scope {
        if constraint.kind != "no_cycles" {
            return Err(format!(
                "invalid spec: {}: [constraint] #{} declares `scope` on a \"{}\" row; `scope` is supported only on `no_cycles` rows",
                spec_path.display(),
                index + 1,
                constraint.kind
            ));
        }
        if scope != "claims" && scope != "both" {
            return Err(format!(
                "invalid spec: {}: constraint #{} has invalid scope \"{}\" (supported: claims, both)",
                spec_path.display(),
                index + 1,
                scope
            ));
        }
    }
    // The `kinds` field of the no_cycles arm (GitHub issue #22): the ring
    // classes the constraint adjudicates. Accepted only on `no_cycles`
    // rows; the two supported values are named in the invalid-value
    // message, same shipped invalid-spec class as the `scope` check above.
    // An absent or empty list is every kind — the shipped behavior.
    if !constraint.kinds.is_empty() {
        if constraint.kind != "no_cycles" {
            return Err(format!(
                "invalid spec: {}: [constraint] #{} declares `kinds` on a \"{}\" row; `kinds` is supported only on `no_cycles` rows",
                spec_path.display(),
                index + 1,
                constraint.kind
            ));
        }
        if let Some(bad) = constraint
            .kinds
            .iter()
            .find(|kind| **kind != "runtime" && **kind != "type-assisted")
        {
            return Err(format!(
                "invalid spec: {}: constraint #{} has invalid kinds value \"{}\" (supported: runtime, type-assisted)",
                spec_path.display(),
                index + 1,
                bad
            ));
        }
    }
    Ok(())
}

/// `contract.expose` is schema at every depth (phase 2, ADR-011): entries are
/// non-empty strings and the parser never consults the tree — an entry's arm
/// (stereotype versus symbol) is decided against model facts at verify, not
/// against spec text (workplan workplan_archspec_module_public_api_diagrams,
/// D1). An empty entry is a mechanical authoring error and is rejected here;
/// semantic validation happens at verify. `contract.forbid` is enforced and
/// stays accepted.
fn validate_contract_expose(spec_path: &std::path::Path, module: &Module) -> Result<(), String> {
    if let Some(entries) = &module.contract.expose
        && let Some(index) = entries.iter().position(|entry| entry.trim().is_empty())
    {
        return Err(format!(
            "invalid spec: {}: module \"{}\" declares an empty contract.expose entry #{}; entries are non-empty globs or stereotype names",
            spec_path.display(),
            module.name,
            index + 1
        ));
    }
    for submodule in &module.submodules {
        validate_contract_expose(spec_path, submodule)?;
    }
    Ok(())
}

/// Claim names are globally unique across all depths (D6): every name also
/// addresses its territory as a `depend_on`/`forbidden` target, so a collision
/// would make a target ambiguous. Rejects at parse, naming both declarations.
fn validate_claim_uniqueness<'a>(
    spec_path: &std::path::Path,
    modules: &'a [Module],
    seen: &mut std::collections::BTreeMap<&'a str, ()>,
) -> Result<(), String> {
    for module in modules {
        if seen.contains_key(module.name.as_str()) {
            return Err(format!(
                "invalid spec: {}: claim name \"{}\" is declared twice; claim names are unique \
                 across all depths, give one of them a distinct name",
                spec_path.display(),
                module.name
            ));
        }
        seen.insert(module.name.as_str(), ());
        validate_claim_uniqueness(spec_path, &module.submodules, seen)?;
    }
    Ok(())
}

/// `matches.units` on a submodule is rejected at parse (D7): the unit tier
/// stays first-declared-wins at the top level, so a nested unit claim would
/// smuggle a second, order-dependent ownership semantics into it.
fn validate_submodule_units(spec_path: &std::path::Path, module: &Module) -> Result<(), String> {
    for submodule in &module.submodules {
        if !submodule.matches.units.is_empty() {
            return Err(format!(
                "invalid spec: {}: submodule \"{}\" declares matches.units; submodules claim \
                 the module tier only",
                spec_path.display(),
                submodule.name
            ));
        }
        validate_submodule_units(spec_path, submodule)?;
    }
    Ok(())
}

/// The tier values one `[[exempt]]` entry may name (GitHub issue #27, R2).
const EXEMPT_TIERS: &[&str] = &["both", "file", "module"];

/// Validate the `[[exempt]]` ratchet ledger (GitHub issue #27): mandatory
/// keys, the tier domain, ring-shaped identities, a positive `count`, a
/// `YYYY-MM-DD` expiry and one entry per (tier, cycle) identity. Every
/// refusal is the shipped `invalid spec: <file>: ...` class naming the
/// entry (1-indexed) and the key — the `validate_constraint` message
/// shape, never a new class.
fn validate_exempt(spec_path: &std::path::Path, entries: &[Exempt]) -> Result<(), String> {
    for (index, entry) in entries.iter().enumerate() {
        let at = |key: &str| {
            Err(format!(
                "invalid spec: {}: [exempt] entry #{}: missing mandatory key \"{key}\" — an \
                 ratchet entry names its ring and the tier it exempts",
                spec_path.display(),
                index + 1,
            ))
        };
        let cycle = match entry.cycle.as_deref().map(str::trim) {
            None => return at("cycle"),
            Some("") => return at("cycle"),
            Some(cycle) => cycle,
        };
        let tier = match entry.tier.as_deref() {
            None | Some("") => return at("tier"),
            Some(tier) => tier,
        };
        if !EXEMPT_TIERS.contains(&tier) {
            return Err(format!(
                "invalid spec: {}: [exempt] entry #{}: tier \"{tier}\" is not supported — \
                 supported values are \"file\", \"module\" and \"both\"",
                spec_path.display(),
                index + 1,
            ));
        }
        // The identity is the ring path the finding states (kind label and
        // tier prefix excluded): at least two distinct stops, head == tail.
        let legs: Vec<&str> = cycle.split(" -> ").map(str::trim).collect();
        if legs.len() < 3
            || legs.iter().any(|leg| leg.is_empty())
            || legs.first() != legs.last()
        {
            return Err(format!(
                "invalid spec: {}: [exempt] entry #{}: cycle \"{cycle}\" is not a ring — \
                 expected \"<member> -> <member> -> ... -> <member>\" with a repeated first \
                 member",
                spec_path.display(),
                index + 1,
            ));
        }
        if entry.count.is_some_and(|count| count < 1) {
            return Err(format!(
                "invalid spec: {}: [exempt] entry #{}: count must be a positive integer — an \
                 entry matching nothing is stale, not capped",
                spec_path.display(),
                index + 1,
            ));
        }
        if let Some(until) = entry.until.as_deref() {
            let digits = |mut range: std::ops::Range<usize>| {
                until.len() == 10 && range.all(|i| until[i..i + 1].bytes().all(|b| b.is_ascii_digit()))
            };
            let date_like = until.is_ascii()
                && digits(0..4)
                && &until[4..5] == "-"
                && digits(5..7)
                && (1..=12).contains(&until[5..7].parse::<u32>().unwrap_or(0))
                && &until[7..8] == "-"
                && digits(8..10)
                && (1..=31).contains(&until[8..10].parse::<u32>().unwrap_or(0));
            if !date_like {
                return Err(format!(
                    "invalid spec: {}: [exempt] entry #{}: until \"{until}\" is not a date — \
                     expected YYYY-MM-DD",
                    spec_path.display(),
                    index + 1,
                ));
            }
        }
        // One identity one entry (the duplicate check runs per pair, spec
        // order first-wins): two entries on one (tier, cycle) would split
        // the cap and the expiry across two truths.
        for (other, other_entry) in entries.iter().enumerate().skip(index + 1) {
            let same = other_entry.cycle.as_deref().map(str::trim) == Some(cycle)
                && other_entry.tier.as_deref() == entry.tier.as_deref();
            if same {
                return Err(format!(
                    "invalid spec: {}: [exempt] entries #{} and #{} are a duplicate exempt \
                     entry for tier \"{tier}\" cycle \"{cycle}\" — one identity one entry",
                    spec_path.display(),
                    index + 1,
                    other + 1,
                ));
            }
        }
    }
    Ok(())
}

/// Basic schema validation over the fields verify reads. Unknown constraint
/// types and empty module names are schema violations naming the file, the
/// offending entry (1-indexed), and why. Extend as later behaviors add checks.
fn validate_schema(spec_path: &std::path::Path, spec: &Spec) -> Result<(), String> {
    // The `[project] framework` gate (round-8 P-FW1): validated against the
    // declared driver's registry through the ONE validator owner. An
    // unparseable language name skips this arm — the shipped unknown-
    // language red owns that value at the verb sites.
    if let Some(name) = spec.project.framework.as_deref()
        && let Some(driver) = language::from_name(&spec.project.language)
        && let Some(finding) = language::framework_error(driver, name)
    {
        return Err(format!("invalid spec: {}: {finding}", spec_path.display()));
    }
    for (index, entry) in spec.project.excluded_sources.iter().enumerate() {
        let name = |key: &str| {
            Err(format!(
                "invalid spec: {}: [project] excluded_sources entry #{} requires a non-empty `{key}` key",
                spec_path.display(),
                index + 1,
            ))
        };
        match (&entry.path, &entry.reason) {
            (None, _) => name("path")?,
            (Some(path), _) if path.trim().is_empty() => name("path")?,
            (_, None) => name("reason")?,
            (_, Some(reason)) if reason.trim().is_empty() => name("reason")?,
            _ => {}
        }
        // Precedence by specificity, no spec-order semantics (decision-check
        // MAJOR-1): two entries whose NORMALISED paths are equal would match
        // every shared file with EQUAL specificity — a tie, and a tie is an
        // invalid-spec error, not a coin flip on TOML order. Path equality
        // normalises separators (`web` and `web/` are one path), matching the
        // component-wise matcher. Distinct paths can never tie (different
        // prefix lengths decide), so this duplicate check is the whole tie
        // law. The message names both entries.
        if let Some(path) = entry.path.as_deref().map(str::trim) {
            for other in index + 1..spec.project.excluded_sources.len() {
                let duplicate = spec.project.excluded_sources[other]
                    .path
                    .as_deref()
                    .is_some_and(|other| std::path::Path::new(other.trim()) == std::path::Path::new(path));
                if duplicate {
                    return Err(format!(
                        "invalid spec: {}: [project] excluded_sources entries #{} and #{} declare the same path \"{path}\" — precedence resolves by specificity and equal specificity is a tie, which is an invalid spec: merge the entries or make one more specific",
                        spec_path.display(),
                        index + 1,
                        other + 1,
                    ));
                }
            }
        }
    }
    for (index, constraint) in spec.constraint.iter().enumerate() {
        validate_constraint(spec_path, constraint, index)?;
    }
    validate_exempt(spec_path, &spec.exempt)?;
    for (index, module) in spec.modules.iter().enumerate() {
        if module.name.trim().is_empty() {
            return Err(format!(
                "invalid spec: {}: module #{} has an empty name",
                spec_path.display(),
                index + 1
            ));
        }
        validate_contract_expose(spec_path, module)?;
        validate_submodule_units(spec_path, module)?;
    }
    let mut seen: std::collections::BTreeMap<&str, ()> = std::collections::BTreeMap::new();
    validate_claim_uniqueness(spec_path, &spec.modules, &mut seen)?;
    let declared: BTreeSet<&str> = spec
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    for (index, constraint) in spec.constraint.iter().enumerate() {
        for name in &constraint.modules {
            if !declared.contains(name.as_str()) {
                return Err(format!(
                    "invalid spec: {}: constraint #{} references undeclared module \"{}\"",
                    spec_path.display(),
                    index + 1,
                    name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_arch_test_kit::model::matching_exclusion;

    /// The shape table drives both refusal messages and schema properties, so
    /// its rows must be sorted (byte-stable output) and duplicate-free.
    #[test]
    fn spec_key_shapes_are_sorted_and_unique() {
        for (shape, keys) in SPEC_KEY_SHAPES {
            let mut sorted = keys.to_vec();
            sorted.sort();
            assert_eq!(sorted, *keys, "shape {shape} must list its keys sorted");
            let unique: std::collections::BTreeSet<&str> = keys.iter().copied().collect();
            assert_eq!(unique.len(), keys.len(), "shape {shape} must list each key once");
        }
    }

    /// The walker refuses undefined keys naming key and location, stays silent
    /// on the valid surface, and never preempts a loader's malformed-TOML
    /// class (issue-8 phase 1, BLOCKER-3).
    #[test]
    fn walker_names_unknown_keys_and_their_locations() {
        let path = std::path::Path::new("architecture.spec.toml");

        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\nexcluded_sourcez = []\n",
            path,
        )
        .expect_err("a typo'd excluded_sources key must be refused");
        assert!(
            err.contains("unknown key(s) \"excluded_sourcez\" in [project]")
                && err.contains("supported keys are \"excluded_sources\""),
            "{err}"
        );

        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored\"\nmood = \"warn\"\n",
            path,
        )
        .expect_err("a typo'd dial key must be refused (phase 1 law intact)");
        assert!(
            err.contains("unknown key(s) \"mood\" in [project] excluded_sources entry #1")
                && err.contains("supported keys are \"mode\", \"path\", \"reason\""),
            "{err}"
        );

        // Phase 2: the dial key is KNOWN, the value domain is fail-closed —
        // an out-of-domain value is refused naming entry, value and values.
        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored\"\nmode = \"report-only\"\n",
            path,
        )
        .expect_err("the retired report-only value must stay refused");
        assert!(
            err.contains("excluded_sources entry #1: mode value \"report-only\" is not supported")
                && err.contains("supported values are \"abort\" and \"warn\""),
            "{err}"
        );
        assert!(
            validate_unknown_keys(
                "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored\"\nmode = \"warn\"\n",
                path,
            )
            .is_ok(),
            "the dial's accepted values must stay silent"
        );

        let err = validate_unknown_keys("[bogus]\nx = 1\n", path).expect_err("stale tables fail");
        assert!(err.contains("unknown key(s) \"bogus\" at the top level"), "{err}");

        assert!(
            validate_unknown_keys("[project]\nlanguage = \"rust\"\n", path).is_ok(),
            "the known surface must stay silent"
        );
        assert!(
            validate_unknown_keys("[project\nlanguage", path).is_ok(),
            "malformed TOML belongs to the loader's parse-error class"
        );
    }

    /// Issue #58: a module declaring `allowed` in BOTH renderings is named,
    /// deterministically, whatever the declaration order and whatever verb
    /// reads the file (every loader shares this walker). An unrelated
    /// duplicate-key error stays silent here — its loader's parse-error
    /// class owns it unchanged (no collateral change to the strictness law).
    #[test]
    fn module_allowed_duplicate_declarations_are_named() {
        let path = std::path::Path::new("architecture.spec.toml");
        let named = "declares \"allowed\" twice (inline key + [module.allowed] section) — keep one";

        // Inline form first, header section second: the parse-failure face.
        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\nallowed = { depend_on = [] }\n\n[module.allowed]\ndepend_on = []\n",
            path,
        )
        .expect_err("inline-then-header on one module must be refused");
        assert!(err.contains("invalid spec:") && err.contains(&format!("module #1 {named}")), "{err}");

        // Header section first, inline key second: the parseable-nesting face
        // (the inline line lands INSIDE the open section — TOML context rules).
        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[module.allowed]\ndepend_on = []\nallowed = { depend_on = [] }\n",
            path,
        )
        .expect_err("header-then-inline on one module must be refused");
        assert!(err.contains("invalid spec:") && err.contains(&format!("module #1 {named}")), "{err}");

        // Deterministic ordinal: the collision on the SECOND module names #2.
        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module]]\nname = \"b\"\nmatches = { units = [\"b\"] }\nallowed = { depend_on = [] }\n\n[module.allowed]\ndepend_on = []\n",
            path,
        )
        .expect_err("the collision must name its own module");
        assert!(err.contains(&format!("module #2 {named}")), "{err}");

        // Submodule face: the collision names the module AND submodule ordinal.
        let err = validate_unknown_keys(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n\n[[module.submodules]]\nname = \"a::b\"\nmatches = { modules = [\"a::b\"] }\nallowed = { depend_on = [] }\n\n[module.submodules.allowed]\ndepend_on = []\n",
            path,
        )
        .expect_err("the submodule collision must be named too");
        assert!(err.contains(&format!("module #1 submodule #1 {named}")), "{err}");

        // No collateral change: an unrelated duplicate-key parse failure stays
        // silent here — the loader's malformed-TOML class owns it byte-for-byte.
        assert!(
            validate_unknown_keys(
                "[project]\nlanguage = \"rust\"\nname = \"x\"\nname = \"y\"\n\n[[module]]\nname = \"a\"\nmatches = { units = [\"a\"] }\n",
                path,
            )
            .is_ok(),
            "an unrelated duplicate key must stay in the loader's parse-error class"
        );
    }

    fn entry(path: &str, mode: &'static str) -> ExclusionEntry {
        ExclusionEntry {
            path: std::path::PathBuf::from(path),
            mode,
        }
    }

    /// Specificity precedence (MAJOR-1): exact beats prefix, the longer
    /// prefix beats the shorter one, and TOML order contributes nothing —
    /// the same match must resolve identically under every declaration
    /// order. Component-wise prefix law: `web` never reaches `webapp`.
    #[test]
    fn exclusion_precedence_resolves_by_specificity_not_order() {
        let exact = entry("src/vendor/legacy.ts", "warn");
        let prefix = entry("src/vendor", "abort");
        let ancestor = entry("src", "abort");

        for order in [
            vec![&exact, &prefix, &ancestor],
            vec![&ancestor, &prefix, &exact],
            vec![&prefix, &ancestor, &exact],
        ] {
            let entries: Vec<ExclusionEntry> = order.iter().map(|e| (*e).clone()).collect();
            let hit = matching_exclusion(&entries, Path::new("src/vendor/legacy.ts"))
                .expect("three ancestors all match the file");
            assert_eq!(hit.path.display().to_string(), "src/vendor/legacy.ts");
            assert_eq!(hit.mode, "warn", "the exact match must win in every order");

            let nested = matching_exclusion(&entries, Path::new("src/vendor/deep/mod.ts")).unwrap();
            assert_eq!(nested.path.display().to_string(), "src/vendor");
            let shallow = matching_exclusion(&entries, Path::new("src/other.ts")).unwrap();
            assert_eq!(shallow.path.display().to_string(), "src");
        }

        let entries = vec![entry("web", "abort")];
        assert_eq!(
            matching_exclusion(&entries, Path::new("web")).map(|h| h.path.as_path()),
            Some(Path::new("web")),
            "the entry path itself matches (directory or file, exact)"
        );
        assert_eq!(
            matching_exclusion(&entries, Path::new("web/app.ts")).map(|h| h.path.as_path()),
            Some(Path::new("web")),
            "component-wise prefix: the entry is a directory root"
        );
        assert!(
            matching_exclusion(&entries, Path::new("webapp/app.ts")).is_none(),
            "component-wise prefix: never a raw character prefix"
        );
    }

    /// The tie law: equal specificity is an invalid-spec error, not a coin
    /// flip on TOML order — two entries declaring the same path (the only
    /// way to tie) are refused naming both entries.
    #[test]
    fn duplicate_exclusion_entries_are_refused_as_the_tie_they_would_cause() {
        let spec: Spec = toml::from_str(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web\"\nreason = \"one\"\n\n[[project.excluded_sources]]\npath = \"web/\"\nreason = \"two\"\n",
        )
        .expect("the TOML parses");
        let err = validate_schema(std::path::Path::new("x"), &spec)
            .expect_err("a tie must be an invalid-spec error");
        assert!(
            err.contains("entries #1 and #2 declare the same path \"web\""),
            "the tie refusal must name both entries and the path: {err}"
        );
    }

    /// The dial's default: an absent `mode` key loads as the shipped
    /// `abort` semantics — today's behavior byte-identical for specs that
    /// never use the key.
    #[test]
    fn absent_mode_defaults_to_abort() {
        let spec: Spec = toml::from_str(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"a\"\nreason = \"r\"\n\n[[project.excluded_sources]]\npath = \"b\"\nreason = \"r\"\nmode = \"warn\"\n",
        )
        .expect("the TOML parses");
        let entries = exclusion_entries(&spec);
        assert_eq!(entries[0].mode, "abort", "the absent key takes the default");
        assert_eq!(entries[1].mode, "warn", "the declared key is kept");
        assert_eq!(
            exclusion_paths(&spec),
            entries.iter().map(|e| e.path.clone()).collect::<Vec<_>>(),
            "exclusion_paths is the path projection of exclusion_entries"
        );
    }

    /// The zero-match liveness line (MAJOR-2): exactly one line per dead
    /// entry; a live entry — file at or under the path — prints none.
    #[test]
    fn liveness_reports_exactly_the_dead_entries_once_each() {
        let tree = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tree.path().join("web/vendor")).expect("dir");
        std::fs::write(tree.path().join("web/vendor/legacy.ts"), "export const x = 1;\n").expect("file");
        let spec: Spec = toml::from_str(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"web/vendor\"\nreason = \"vendored\"\n\n[[project.excluded_sources]]\npath = \"tools/generated\"\nreason = \"generated\"\n",
        )
        .expect("the TOML parses");
        let report = exclusion_liveness_report(tree.path(), &spec);
        assert_eq!(
            report,
            "excluded source matched nothing: tools/generated — generated\n",
            "the dead entry prints exactly one line, the live one prints nothing"
        );
    }

    /// Liveness behind the mechanical exclusion set: a subtree the walk
    /// never visits (here: a `target` directory) cannot keep a dead entry
    /// alive — the silent-inert class stays loud.
    #[test]
    fn liveness_walks_behind_the_mechanical_exclusion_set() {
        let tree = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tree.path().join("target/gen")).expect("dir");
        std::fs::write(tree.path().join("target/gen/lib.rs"), "// generated\n").expect("file");
        let spec: Spec = toml::from_str(
            "[project]\nlanguage = \"rust\"\n\n[[project.excluded_sources]]\npath = \"target/gen\"\nreason = \"build output\"\n",
        )
        .expect("the TOML parses");
        let report = exclusion_liveness_report(tree.path(), &spec);
        assert_eq!(
            report,
            "excluded source matched nothing: target/gen — build output\n",
            "files only inside the mechanical set must not count as matched: {report}"
        );
    }

    fn constraint(kind: &str) -> Constraint {
        Constraint {
            kind: kind.to_string(),
            modules: Vec::new(),
            severity: String::new(),
            scope: None,
            kinds: Vec::new(),
            allowed: Vec::new(),
            from: Vec::new(),
            forbid: Vec::new(),
            require_publish: None,
            forbidden_dependencies: Vec::new(),
            required_features: Vec::new(),
            feature: String::new(),
            gated_modules: Vec::new(),
            allowed_from: Vec::new(),
            parent: String::new(),
        }
    }

    fn set_field(constraint: &mut Constraint, field: &str, present: bool) {
        match field {
            "allowed" => constraint.allowed = if present { vec!["a".into()] } else { Vec::new() },
            "from" => constraint.from = if present { vec!["a".into()] } else { Vec::new() },
            "forbid" => constraint.forbid = if present { vec!["a".into()] } else { Vec::new() },
            "gated_modules" => {
                constraint.gated_modules = if present { vec!["a".into()] } else { Vec::new() };
            }
            "feature" => constraint.feature = if present { "f".into() } else { String::new() },
            "parent" => constraint.parent = if present { "p".into() } else { String::new() },
            _ => {}
        }
    }

    #[test]
    fn validator_enforces_shared_required_table() {
        for (kind, fields) in CONSTRAINT_REQUIRED_FIELDS {
            let mut full = constraint(kind);
            for field in *fields {
                set_field(&mut full, field, true);
            }
            assert!(
                validate_constraint(std::path::Path::new("x"), &full, 0).is_ok(),
                "{kind} with all required fields present must validate"
            );

            for field in *fields {
                let mut missing = constraint(kind);
                for other in *fields {
                    set_field(&mut missing, other, true);
                }
                set_field(&mut missing, field, false);
                let err = validate_constraint(std::path::Path::new("x"), &missing, 0)
                    .expect_err("missing required field must fail validation");
                assert!(
                    err.contains(&format!("`{field}`")),
                    "{kind} missing {field} must name it: {err}"
                );
            }
        }
    }
}

