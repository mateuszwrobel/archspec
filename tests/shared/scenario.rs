use crate::common;
use crate::shared::driver::Driver;
use crate::shared::feature::Feature;

/// How a scenario decides which drivers it exercises.
pub enum Applies {
    /// Capability skip driven by the library's capability table: the scenario
    /// runs exactly where the driver emits the cited fact at full granularity
    /// (`granular`). No per-language list lives here — the table decides, and
    /// the prose guard (`tests/capability_prose.rs`) checks the citation.
    Capability(&'static str),
    /// Capability skip for the complement: the scenario runs exactly where the
    /// driver does NOT emit the cited fact at full granularity — a behavior
    /// pinned to the inert side of the table (vacuity announcements, honest
    /// "not verifiable" wordings).
    Inert(&'static str),
    /// A non-capability applicability rule (tree shape, materializer detail).
    Custom(fn(&Driver) -> bool),
}

/// A granular behavior assertion for one feature, run against every driver.
pub struct Scenario {
    /// The capability column this behavior belongs to.
    pub feature: Feature,
    /// Granular behavior, snake_case, e.g. `lists_file_scoped_namespaces`.
    pub name: &'static str,
    /// Human sentence, e.g. `a file-scoped namespace X.Y; becomes a soft_structure entry`.
    pub description: &'static str,
    /// The actual behavior assertion. `Ok(())` passes; `Err` fails the scenario.
    pub run: fn(&Driver, &common::Fixture) -> Result<(), String>,
    /// Whether the behavior is exercisable on a driver. `None` means the
    /// behavior applies on every driver that implements the feature; `Some`
    /// limits it (capability-driven skips cite the capability table's fact
    /// name; custom predicates encode tree shapes the table does not state).
    /// A scenario that does not apply renders `skipped`, never `pass`.
    pub applies: Option<Applies>,
}