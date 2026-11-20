use crate::model::{DeclaredFacts, Model};
use std::path::Path;

/// The driver-port seam (workplan events_value_join scenario 10, ADR-069
/// Decision 5's ports rung): the compiler-checked handshake between the scan
/// host and the language drivers. ONE object-safe trait, ONE method —
/// today's extract shape, so every driver implements it by calling its own
/// `extract` unchanged and body logic moves zero lines. The compile-time
/// table in `crate::scan` (the `driver` fn) is its only user: six unit-struct
/// drivers, exhaustive by construction. No runtime registration exists and
/// none may ship: no register verb, no `Vec` of boxes, no lock-guarded
/// lookup — the table is compile-time by ADR-069 Alternatives (in-process
/// dlopen REJECTED).
///
/// The literal candidates ride [`Model::literal_candidates`] exactly as they
/// do today. The literal-site join stays the ONE engine-side site
/// (`archspec::scan`'s gated block), and this port must not grow toward it
/// or toward the events machinery (decision-check A4). External plugins are
/// SUBPROCESS FactDoc producers that bypass `extract_with` entirely
/// (ADR-069 Decision 2, an evidence-gated null): this trait is the
/// engine-internal seam — a compile-checked handshake shape, nothing more.
pub(crate) trait Driver {
    /// Extract the architecture model of `root` for this driver's language,
    /// adjudicated against the declared facts the consumer loaded from its
    /// own spec (the empty set is the spec-free arm).
    fn extract(&self, root: &Path, declared: &DeclaredFacts) -> Result<Model, String>;
}
