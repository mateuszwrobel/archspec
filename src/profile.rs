//! Feature-gated phase-timing seam (GitHub issue #34).
//!
//! The pipeline phase boundaries in `commands::verify`, `verify::compare`
//! and the language drivers' fact legs call [`phase`] around the work they
//! already do. With the `profile-phases` cargo feature off — every gate run
//! and every shipped build — the call inlines to the wrapped closure alone:
//! no clock, no record, no label string survives into the binary (the
//! graph-level proof lives in `tests/profile_gate.rs`; the binary target
//! additionally refuses the feature at compile time).
//!
//! With the feature on, each call records one span (label, wall duration)
//! into a process-global buffer that the dev-only `phase-probe` example
//! drains between runs. The recorder imports nothing outside `std` and the
//! phase calls cross module boundaries only into foundations, so the
//! dependency layers in `architecture.spec.toml` are untouched.

/// One record per phase call: the boundary's label and its wall duration
/// in nanoseconds, in call order. A poisoned lock yields an empty run of
/// records instead of a panic: profiling is never a correctness surface.
#[cfg(feature = "profile-phases")]
static SPANS: std::sync::Mutex<Vec<(&'static str, u128)>> = std::sync::Mutex::new(Vec::new());

/// Run `f` as the named pipeline phase. Feature off: exactly `f()`.
#[inline]
pub fn phase<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    #[cfg(feature = "profile-phases")]
    {
        let start = std::time::Instant::now();
        let value = f();
        if let Ok(mut spans) = SPANS.lock() {
            spans.push((name, start.elapsed().as_nanos()));
        }
        value
    }
    #[cfg(not(feature = "profile-phases"))]
    {
        let _ = name;
        f()
    }
}

/// Drop the spans recorded so far (the harness calls this between runs).
#[cfg(feature = "profile-phases")]
pub fn reset() {
    if let Ok(mut spans) = SPANS.lock() {
        spans.clear();
    }
}

/// The spans recorded since the last [`reset`], in call order. Nested
/// boundaries appear as their own entries next to their parents — the
/// harness treats the declared top-level phases as the total and every
/// nested one as a share of it.
#[cfg(feature = "profile-phases")]
#[must_use]
pub fn snapshot() -> Vec<(&'static str, u128)> {
    if let Ok(spans) = SPANS.lock() {
        return spans.clone();
    }
    Vec::new()
}
