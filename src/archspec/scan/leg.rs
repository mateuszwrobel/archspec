//! The parallel fact-leg leaf (self-audit F1): the ordered-slot machinery the
//! language drivers share. It imports NOTHING from the scan module that hosts
//! it — the drivers call down into this leaf, the host never calls back — the
//! shape that keeps `scan` and its drivers out of one cycle (the recurrence
//! gate: `architecture.spec.toml` scan-core/scan-leg rows + `no_cycles`).

/// Parallel fact-leg floor (round 9, P-PAR2): with fewer files than this
/// the drivers' per-file fact leg runs LITERALLY today's serial loop — no
/// threads spawn, so test processes and small trees stay cold.
pub(crate) const PARALLEL_FACT_LEG_FLOOR: usize = 128;

/// Worker count for the pure per-file fact leg (round 9, P-PAR2):
/// `min(available_parallelism, files_len)`, serial below the floor
/// (`available_parallelism` is cgroup-aware natively — the oversubscription
/// defense a parallel runtime would sell is already native here). The hidden
/// `ARCHSPEC_THREADS` env is the serial-vs-parallel A/B lever: test control
/// only, NOT public config, NOT documented surface — the shape follows the
/// `ARCHSPEC_DISABLE_DRIVERS` precedent above.
pub(crate) fn fact_leg_threads(files_len: usize) -> usize {
    if files_len < PARALLEL_FACT_LEG_FLOOR {
        return 1;
    }
    let requested = std::env::var("ARCHSPEC_THREADS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|count| *count > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|width| width.get())
                .unwrap_or(1)
        });
    files_len.min(requested)
}

/// The ordered slot vector of the parallel fact leg (round 9, F13): runs
/// `facts` for every index on `threads` workers with an atomic chunk-claim
/// loop over `std::thread::scope`, and returns `files_len` results AT THE
/// WALK-INDEX positions — the fold replays them in index order. The closure
/// must be the PURE per-file leg (read + parse, no engine contact, no
/// print): the engine stays `!Send + !Sync` by design, the type system is
/// the guard that keeps it out of these threads (F14 law).
pub(crate) fn parallel_fact_leg<T, F>(
    files_len: usize,
    threads: usize,
    facts: F,
) -> Vec<Result<T, String>>
where
    T: Send + Sync,
    F: Fn(usize) -> Result<T, String> + Sync,
{
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let slots: Vec<OnceLock<Result<T, String>>> =
        (0..files_len).map(|_| OnceLock::new()).collect();
    let next = AtomicUsize::new(0);
    let chunk = (files_len / (threads * 8)).max(1);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let start = next.fetch_add(chunk, Ordering::Relaxed);
                if start >= files_len {
                    break;
                }
                let end = (start + chunk).min(files_len);
                for (index, slot) in slots.iter().enumerate().take(end).skip(start) {
                    let _ = slot.set(facts(index));
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|slot| slot.into_inner().expect("fact slot left unfilled"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PARALLEL_FACT_LEG_FLOOR, fact_leg_threads};

#[test]
    fn fact_leg_floor_keeps_small_trees_serial() {
        assert_eq!(fact_leg_threads(PARALLEL_FACT_LEG_FLOOR - 1), 1);
        // The floor wins over the lever: below it no threads are spawned.
        unsafe { std::env::set_var("ARCHSPEC_THREADS", "8") };
        assert_eq!(fact_leg_threads(PARALLEL_FACT_LEG_FLOOR - 1), 1);
        unsafe { std::env::remove_var("ARCHSPEC_THREADS") };
    }

    #[test]
    fn fact_leg_threads_lever_forces_serial_and_parallel() {
        unsafe { std::env::set_var("ARCHSPEC_THREADS", "1") };
        assert_eq!(fact_leg_threads(4096), 1);
        unsafe { std::env::set_var("ARCHSPEC_THREADS", "8") };
        assert_eq!(fact_leg_threads(PARALLEL_FACT_LEG_FLOOR), 8);
        unsafe { std::env::remove_var("ARCHSPEC_THREADS") };
    }

    #[test]
    fn fact_leg_threads_never_exceed_files() {
        // The lever names the worker count directly (A/B oversubscription);
        // only the file count caps it.
        unsafe { std::env::set_var("ARCHSPEC_THREADS", "9000") };
        assert_eq!(fact_leg_threads(PARALLEL_FACT_LEG_FLOOR), PARALLEL_FACT_LEG_FLOOR);
        unsafe { std::env::remove_var("ARCHSPEC_THREADS") };
    }

    #[test]
    fn fact_leg_threads_defaults_to_parallelism_at_least_floor() {
        unsafe { std::env::remove_var("ARCHSPEC_THREADS") };
        let threads = fact_leg_threads(PARALLEL_FACT_LEG_FLOOR);
        let width = std::thread::available_parallelism()
            .map(|width| width.get())
            .unwrap_or(1);
        assert_eq!(threads, width.min(PARALLEL_FACT_LEG_FLOOR));
    }
}
